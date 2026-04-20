#[cfg(feature = "cloud")]
use std::net::IpAddr;
use std::time::Duration;

use axum::{http::StatusCode, response::IntoResponse};
use axum_typed_websockets::{Message, WebSocket};
#[cfg(feature = "cloud")]
use cloudflare::{
	endpoints::{
		cfd_tunnel::*,
		dns::dns::*,
		workerskv::{read_key, write_key},
		zones::zone::*,
	},
	framework::{
		Environment,
		OrderDirection,
		SearchMatch,
		auth::Credentials,
		client::{ClientConfig, async_api::Client as CloudflareClient},
		response::ApiSuccess,
	},
};
use futures::{
	future::{self, Either},
	prelude::stream::*,
};
#[cfg(feature = "cloud")]
use models::cloudflare::kv::InternalKVData;
use models::{
	api::workspace::{
		deployment::DeploymentStatus,
		runner::{StreamRunnerDataForWorkspaceClientMsg::*, *},
	},
	utils::{GenericResponse, WebSocketUpgrade},
};
use rustis::{
	ClientError,
	Error as RedisError,
	client::{BatchPreparedCommand as _, Client as RedisClient},
	commands::{
		GenericCommands,
		SetCondition,
		SetExpiration,
		StringCommands,
		TransactionCommands as _,
	},
};
use tokio_util::sync::CancellationToken;

use crate::prelude::*;

pub async fn stream_runner_data_for_workspace(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: StreamRunnerDataForWorkspacePath {
					workspace_id,
					runner_id,
				},
				query: (),
				headers:
					StreamRunnerDataForWorkspaceRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: WebSocketUpgrade(upgrade),
			},
		database: _,
		redis,
		client_ip: _,
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, StreamRunnerDataForWorkspaceRequest>,
) -> Result<AppResponse<StreamRunnerDataForWorkspaceRequest>, ErrorType> {
	// Try to acquire a lock on redis first
	let random_connection_id = Uuid::new_v4();
	let Ok(true) = redis
		.set_with_options(
			redis::keys::runner_connection_lock(&runner_id),
			random_connection_id.to_string(),
			SetCondition::NX,
			SetExpiration::Ex(
				const {
					if cfg!(debug_assertions) {
						5 // 5 seconds
					} else {
						120 // 2 mins
					}
				},
			),
		)
		.await
	else {
		return Err(ErrorType::RunnerAlreadyConnected);
	};

	let redis = redis.clone();

	AppResponse::builder()
		.body(GenericResponse(
			upgrade
				.on_upgrade(async move |websocket| {
					handle_websocket(
						websocket,
						workspace_id,
						runner_id,
						redis,
						random_connection_id,
						state,
					)
					.await;
				})
				.into_response(),
		))
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn handle_websocket(
	mut websocket: WebSocket<
		StreamRunnerDataForWorkspaceServerMsg,
		StreamRunnerDataForWorkspaceClientMsg,
	>,
	workspace_id: Uuid,
	runner_id: Uuid,
	redis: RedisClient,
	random_connection_id: Uuid,
	state: AppState,
) {
	let exposure_type;
	let runner_version;

	loop {
		let Some(message) = websocket.next().await else {
			debug!("Websocket client disconnected before sending handshake");
			return;
		};
		let Ok(Message::Item(message)) = message else {
			// Error on websocket, continue to try again
			continue;
		};

		trace!("Received message from websocket: {:?}", message);
		let Handshake {
			version,
			exposure_type: new_exposure_type,
		} = message
		else {
			// Ignore other messages until the handshake is received
			let Ok(()) = websocket
				.send(Message::Item(
					StreamRunnerDataForWorkspaceServerMsg::HandshakeRequired,
				))
				.await
			else {
				debug!("Failed to send handshake required message to websocket");
				continue;
			};
			continue;
		};

		exposure_type = new_exposure_type;
		runner_version = version;

		break;
	}

	let Ok(()) = update_runner_exposure_type(runner_id, workspace_id, exposure_type, &state)
		.await
		.inspect_err(|err| {
			error!(
				"Failed to update runner exposure type for runner ID: {}: {:?}",
				runner_id, err
			);
		})
	else {
		error!("Failed to update runner exposure type for runner ID: {runner_id}");
		return;
	};

	let Ok(_) = query!(
		r#"
		UPDATE
			runner
		SET
			is_connected = TRUE,
			last_seen = NOW(),
			version = $2
		WHERE
			id = $1;
		"#,
		runner_id as _,
		runner_version.to_string(),
	)
	.execute(&state.database)
	.await
	.inspect_err(|err| error!("Failed to set runner as connected: {:?}", err)) else {
		return;
	};

	let redis_channel = format!("{workspace_id}/runner/{runner_id}/stream");
	let mut pub_sub = redis.create_pub_sub();

	match pub_sub.subscribe(&redis_channel).await {
		Ok(()) => (),
		Err(RedisError::Client(ClientError::AlreadySubscribed)) => {
			warn!("Already subscribed to the runner data stream.");
			debug!(concat!(
				"This only happens when a previous loop ",
				"of this function tried to connect and subscribed. ",
				"Ignore the error"
			));
		}
		Err(err) => {
			error!("Error streaming runner data: {:?}", err);
			return;
		}
	}

	let ping_interval = if cfg!(debug_assertions) {
		Duration::from_secs(1)
	} else {
		Duration::from_secs(30)
	};

	let mut sleeper = Box::pin(tokio::time::sleep(ping_interval));

	loop {
		// Make sure to check for cancellation
		let Some(actionable_future) = future::select(
			future::select(
				future::select(pub_sub.next(), websocket.next()),
				&mut sleeper,
			),
			std::pin::pin!(
				crate::GLOBAL_CANCEL_TOKEN
					.get_or_init(CancellationToken::new)
					.cancelled()
			),
		)
		.await
		.into_left() else {
			// Global cancellation triggered
			debug!("Global cancellation triggered, closing websocket");
			break;
		};

		let Some(reader_writer) = actionable_future.into_left() else {
			// Reset the sleeper for the next ping interval
			sleeper = Box::pin(tokio::time::sleep(ping_interval));

			let Ok(_) = websocket.send(Message::Ping(Default::default())).await else {
				debug!("Failed to send ping to websocket");
				break;
			};
			// Compare-and-renew: WATCH the lock, GET to confirm we still own it,
			// then MULTI/PEXPIRE/EXEC to extend the TTL. EXEC returns
			// `Error::Aborted` if anything (including our own client) wrote
			// to the key between WATCH and EXEC, in which case we bail
			// without touching the lock further. Replaces the previous
			// `SET XX` which would happily extend another runner's lock
			// after a TTL-expiry race.
			let lock_key = redis::keys::runner_connection_lock(&runner_id);
			let my_uuid = random_connection_id.to_string();
			let ttl_seconds: u64 = if cfg!(debug_assertions) { 5 } else { 120 };

			if redis.watch(lock_key.clone()).await.is_err() {
				info!("Failed to WATCH runner connection lock, closing websocket");
				break;
			}
			let current = match redis.get::<Option<String>>(lock_key.clone()).await {
				Ok(v) => v,
				Err(_) => {
					let _ = redis.unwatch().await;
					info!("Failed to GET runner connection lock, closing websocket");
					break;
				}
			};
			if current.as_deref() != Some(my_uuid.as_str()) {
				let _ = redis.unwatch().await;
				info!("Runner connection lock no longer owned by us, closing websocket");
				break;
			}

			let mut tx = redis.create_transaction();
			tx.pexpire(lock_key.clone(), ttl_seconds * 1000, None)
				.queue();
			match tx.execute::<()>().await {
				Ok(_) => {}
				Err(RedisError::Aborted) => {
					info!("Runner connection lock changed during renewal, closing websocket");
					break;
				}
				Err(err) => {
					error!("Failed to renew runner connection lock: {:?}", err);
					break;
				}
			}
			continue;
		};

		match reader_writer {
			Either::Left((publish_data, _)) => {
				let Some(data) = publish_data else {
					// pub_sub stream ended
					debug!("Redis pub/sub stream ended");
					break;
				};
				let Ok(data) = data else {
					// Error on pub_sub, continue to try again
					continue;
				};
				let Ok(data) = serde_json::from_slice(&data.payload)
					.inspect_err(|err| error!("Error streaming runner data: {:?}", err))
				else {
					break;
				};
				trace!("Sending data down the pipe: {:?}", data);
				let Ok(_) = websocket.send(Message::Item(data)).await else {
					debug!("Failed to send data to websocket");
					break;
				};
			}
			Either::Right((client_message, _)) => {
				let Some(message) = client_message else {
					// Websocket stream ended (client disconnected)
					debug!("Websocket client disconnected");
					break;
				};
				let Ok(message) = message else {
					// Error on websocket, continue to try again
					continue;
				};
				let Message::Item(message) = message else {
					continue;
				};
				trace!("Received message from websocket: {:?}", message);

				match message {
					DeploymentStatusUpdated { id, status } => {
						let Ok(()) = update_deployment_status(id, runner_id, status, &state)
							.await
							.inspect_err(|err| {
								error!(
									"Failed to update deployment status for deployment ID: {}: {:?}",
									id, err
								);
							})
						else {
							error!("Failed to update deployment status for deployment ID: {id}");
							continue;
						};
					}
					Handshake {
						version,
						exposure_type,
					} => {
						let Ok(()) = update_runner_exposure_type(
							runner_id,
							workspace_id,
							exposure_type,
							&state,
						)
						.await
						.inspect_err(|err| {
							error!(
								"Failed to update runner exposure type for runner ID: {}: {:?}",
								runner_id, err
							);
						}) else {
							error!(
								"Failed to update runner exposure type for runner ID: {runner_id}"
							);
							continue;
						};

						let Ok(_) = query!(
							r#"
							UPDATE
								runner
							SET
								version = $2
							WHERE
								id = $1;
							"#,
							runner_id as _,
							version.to_string(),
						)
						.execute(&state.database)
						.await
						.inspect_err(|err| error!("Failed to update runner version: {:?}", err)) else {
							continue;
						};
					}
				}
			}
		}
	}

	_ = query!(
		r#"
		UPDATE
			runner
		SET
			is_connected = FALSE,
			last_seen = NOW()
		WHERE
			id = $1;
		"#,
		runner_id as _,
	)
	.execute(&state.database)
	.await
	.inspect_err(|err| error!("Failed to set runner as disconnected: {:?}", err));

	trace!("Websocket closed, unsubscribing from runner data stream");
	_ = pub_sub
		.unsubscribe(&redis_channel)
		.await
		.inspect_err(|err| error!("Error streaming runner data: {:?}", err));
	// Compare-and-delete on shutdown so we never wipe a lock that another
	// runner has since taken (the original blip-and-resume sequence the
	// previous unconditional DEL was vulnerable to). Aborted just means
	// someone else has the lock now — also fine.
	let lock_key = redis::keys::runner_connection_lock(&runner_id);
	let my_uuid = random_connection_id.to_string();
	let release_result = async {
		redis.watch(lock_key.clone()).await?;
		let current: Option<String> = redis.get(lock_key.clone()).await?;
		if current.as_deref() != Some(my_uuid.as_str()) {
			return redis.unwatch().await;
		}
		let mut tx = redis.create_transaction();
		tx.del(lock_key.clone()).queue();
		match tx.execute::<()>().await {
			Ok(_) | Err(RedisError::Aborted) => Ok(()),
			Err(e) => Err(e),
		}
	}
	.await;
	if let Err(err) = release_result {
		error!("Error releasing runner connection lock: {:?}", err);
	}
	_ = websocket.close().await;
}

async fn update_deployment_status(
	id: Uuid,
	runner_id: Uuid,
	status: DeploymentStatus,
	state: &AppState,
) -> Result<(), ErrorType> {
	// Try to update the status, then always return the current status
	let current_status = query!(
		r#"
		WITH updated AS (
			UPDATE
				deployment
			SET
				status = $1
			WHERE
				id = $2 AND
				status != $1 AND (
					(
						$1 = 'errored' AND status = 'deploying'
					) OR (
						$1 = 'deploying' AND status = 'errored'
					) OR (
						$1 IN ('deploying', 'errored') AND status = 'running'
					) OR (
						$1 = 'running' AND status IN ('deploying', 'errored')
					)
				)
			RETURNING status
		)
		SELECT COALESCE(
			(
				SELECT status FROM updated
			),
			(
				SELECT
					status
				FROM
					deployment
				WHERE
					id = $2
			)
		) AS "status!: DeploymentStatus";
		"#,
		status as _,
		id as _,
	)
	.fetch_one(&state.database)
	.await?
	.status;

	cfg_if! {
		if #[cfg(feature = "cloud")] {
			let client = CloudflareClient::new(
				Credentials::UserAuthToken {
					token: state.config.cloudflare.api_key.clone(),
				},
				ClientConfig::default(),
				Environment::Custom(state.config.cloudflare.base_url.clone()),
			)?;

			// Read existing KV to get the ports
			let existing_kv = serde_json::from_slice::<InternalKVData>(
				&client
					.request(&read_key::ReadKey {
						account_identifier: &state.config.cloudflare.account_id,
						namespace_identifier: &state.config.cloudflare.worker_namespace_id,
						key: &id.to_string(),
					})
					.await?,
			)?;

			let InternalKVData::Deployment { ports, .. } = &existing_kv else {
				return Err(ErrorType::server_error(
					"expected deployment KV data, found runner",
				));
			};

			client
				.request(&write_key::WriteKey {
					account_identifier: &state.config.cloudflare.account_id,
					namespace_identifier: &state.config.cloudflare.worker_namespace_id,
					key: &id.to_string(),
					params: write_key::WriteKeyParams {
						expiration: None,
						expiration_ttl: None,
					},
					body: write_key::WriteKeyBody::Value(serde_json::to_vec(
						&InternalKVData::Deployment {
							ports: ports.clone(),
							runner_id,
							status: current_status,
						},
					)?),
				})
				.await?;
		} else {
			let _ = (runner_id, current_status);
		}
	}

	Ok(())
}

async fn update_runner_exposure_type(
	runner_id: Uuid,
	workspace_id: Uuid,
	exposure_type: RunnerExposureType,
	state: &AppState,
) -> Result<(), ErrorType> {
	cfg_if! {
		if #[cfg(feature = "cloud")] {
	let client = CloudflareClient::new(
		Credentials::UserAuthToken {
			token: state.config.cloudflare.api_key.clone(),
		},
		Default::default(),
		Environment::Custom(state.config.cloudflare.base_url.clone()),
	)?;

	let zone_id = client
		.request(&ListZones {
			params: ListZonesParams {
				name: Some(state.config.primary_hosted_domain.clone()),
				status: Some(Status::Active),
				search_match: Some(SearchMatch::All),
				..Default::default()
			},
		})
		.await?
		.result
		.into_iter()
		.next()
		.ok_or(ErrorType::ResourceDoesNotExist)
		.inspect_err(|_| {
			error!(
				"No zone exists for the domain `{}`",
				state.config.primary_hosted_domain
			);
		})?
		.id;

	let mut records_to_create = match exposure_type {
		RunnerExposureType::Private => {
			trace!("Updating DNS record for the tunnel");

			let tunnel_id = query!(
				r#"
				SELECT
					*
				FROM
					runner
				WHERE
					id = $1 AND
					workspace_id = $2 AND
					deleted IS NULL;
				"#,
				&runner_id as _,
				&workspace_id as _,
			)
			.fetch_optional(&state.database)
			.await?
			.ok_or(ErrorType::ResourceDoesNotExist)?
			.cloudflare_tunnel_id;

			let tunnel = reqwest::Client::new()
				.get(format!(
					"{}accounts/{}/cfd_tunnel/{}",
					state.config.cloudflare.base_url, state.config.cloudflare.account_id, tunnel_id
				))
				.bearer_auth(&state.config.cloudflare.api_key)
				.send()
				.await?
				.json::<ApiSuccess<Option<Tunnel>>>()
				.await?
				.result
				.filter(|tunnel| tunnel.deleted_at.is_none());

			let tunnel = if let Some(tunnel) = tunnel {
				info!("Tunnel exists. Updating tunnel `{}`", tunnel.id);
				tunnel
			} else {
				// The tunnel does not exist. Create one
				info!("Tunnel does not exist. Creating tunnel");
				let tunnel = client
					.request(&create_tunnel::CreateTunnel {
						account_identifier: &state.config.cloudflare.account_id,
						params: create_tunnel::Params {
							config_src: &ConfigurationSrc::Cloudflare,
							name: &format!("Runner: {}", runner_id),
							tunnel_secret: &b"default".to_vec(),
							metadata: None,
						},
					})
					.await?
					.result;

				query!(
					r#"
					UPDATE
						runner
					SET
						cloudflare_tunnel_id = $1
					WHERE
						id = $2 AND
						workspace_id = $3 AND
						deleted IS NULL;
					"#,
					&tunnel.id as _,
					&runner_id as _,
					&workspace_id as _,
				)
				.execute(&state.database)
				.await?;

				tunnel
			};

			vec![DnsContent::CNAME {
				content: format!("{}.cfargotunnel.com", tunnel.id),
			}]
		}
		RunnerExposureType::PublicIP { mut ip_addresses } => {
			ip_addresses.sort();

			ip_addresses
				.into_iter()
				.map(|ip| match ip {
					IpAddr::V4(ip) => DnsContent::A { content: ip },
					IpAddr::V6(ip) => DnsContent::AAAA { content: ip },
				})
				.collect::<Vec<_>>()
		}
		RunnerExposureType::PublicDNS { dns_name } => vec![DnsContent::CNAME { content: dns_name }],
	}
	.into_iter();

	let existing_records = client
		.request(&ListDnsRecords {
			zone_identifier: &zone_id,
			params: ListDnsRecordsParams {
				name: Some(format!(
					"{}.{}",
					runner_id, state.config.primary_hosted_domain
				)),
				order: Some(ListDnsRecordsOrder::Content),
				direction: Some(OrderDirection::Ascending),
				per_page: Some(100),
				..Default::default()
			},
		})
		.await?
		.result;

	// Update all existing records with the new content
	for existing_record in existing_records {
		let new_record = records_to_create.next();
		if let Some(content) = new_record {
			debug!(
				"Updating existing DNS record `{}` for runner `{}`",
				existing_record.id, runner_id
			);
			client
				.request(&UpdateDnsRecord {
					zone_identifier: &zone_id,
					identifier: &existing_record.id,
					params: UpdateDnsRecordParams {
						name: &format!("{}.{}", runner_id, state.config.primary_hosted_domain),
						ttl: Some(0),
						proxied: Some(true),
						content,
					},
				})
				.await?;
		} else {
			debug!(
				"Deleting extra DNS record `{}` for runner `{}`",
				existing_record.id, runner_id
			);
			client
				.request(&DeleteDnsRecord {
					zone_identifier: &zone_id,
					identifier: &existing_record.id,
				})
				.await?;
		}
	}

	// If there are still pending DNS records to be updated, create them
	for content in records_to_create {
		info!("DNS record for the runner does not exist. Creating a new one");
		client
			.request(&CreateDnsRecord {
				zone_identifier: &zone_id,
				params: CreateDnsRecordParams {
					name: &format!("{}.{}", runner_id, state.config.primary_hosted_domain),
					ttl: Some(0),
					proxied: Some(true),
					priority: None,
					content,
				},
			})
			.await?;
	}
		} else {
			let _ = (runner_id, workspace_id, exposure_type, state);
		}
	}

	Ok(())
}
