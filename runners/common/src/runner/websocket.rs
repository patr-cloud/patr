use std::{
	collections::{BTreeMap, BTreeSet},
	fmt::Debug,
};

use ::time::OffsetDateTime;
use futures::{
	Sink,
	SinkExt,
	Stream,
	StreamExt,
	future::{self, Either},
};
use http::StatusCode;
use models::{
	api::workspace::{
		deployment::*,
		domain::{
			GetDomainInfoInWorkspacePath,
			GetDomainInfoInWorkspaceRequest,
			GetDomainInfoInWorkspaceRequestHeaders,
		},
		managed_url::{
			ListManagedURLPath,
			ListManagedURLRequest,
			ListManagedURLRequestHeaders,
			ManagedUrlType,
		},
		runner::*,
		secret::{
			ListSecretsForWorkspacePath,
			ListSecretsForWorkspaceRequest,
			ListSecretsForWorkspaceRequestHeaders,
		},
	},
	utils::ResourceSearcher,
};
use tokio::{
	sync::mpsc,
	time::{self, Duration},
};

use super::{
	controller::ControllerMessage,
	db_helpers::{self, get_local_deployment_info},
};
use crate::prelude::*;

/// Messages for the WebSocket connection.
#[derive(Debug)]
pub enum WebSocketMessage {
	/// The controller reports a status change to send upstream. Dropped while
	/// disconnected, since every resync sends them all.
	SendStatusUpdate {
		/// The UUID of the resource whose status changed.
		resource_id: Uuid,
		/// The new status to report upstream.
		status: ObservedDeploymentStatus,
	},
}

/// How often the local database is resynced with the upstream APIs. 30 seconds
/// in debug mode.
const FULL_RESYNC_INTERVAL: Duration = if cfg!(debug_assertions) {
	Duration::from_secs(30)
} else {
	Duration::from_mins(15)
};

/// How long connecting to the server can take before it counts as failed.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// Connects to the server and sends the handshake. The server streams nothing,
/// and shows the runner as disconnected, until it has the handshake, so it's
/// sent before the connection is used for anything else.
pub async fn connect<E: RunnerExecutor>(
	config: &RunnerSettings<E::Settings>,
) -> Result<
	impl Stream<Item = Result<StreamRunnerDataForWorkspaceServerMsg, ErrorType>>
	+ Sink<StreamRunnerDataForWorkspaceClientMsg, Error: Debug>
	+ Unpin
	+ use<E>,
	RunnerError,
> {
	let RunnerSettings {
		workspace_id,
		runner_id,
		api_token,
		user_agent,
		..
	} = config.clone();

	info!("Connecting to the server");
	let response = time::timeout(
		CONNECT_TIMEOUT,
		client::stream_request(
			ApiRequest::<StreamRunnerDataForWorkspaceRequest>::builder()
				.path(StreamRunnerDataForWorkspacePath {
					workspace_id,
					runner_id,
				})
				.headers(StreamRunnerDataForWorkspaceRequestHeaders {
					authorization: api_token,
					user_agent,
				})
				.build(),
		),
	)
	.await;

	let mut stream = match response {
		Ok(Ok(stream)) => stream,
		Ok(Err(err)) => {
			// Keep retrying even when the fix is out of our hands: exiting would
			// only have systemd restart us into the same rejection.
			match (err.status_code, &err.body.error) {
				(StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN, _) |
				(_, ErrorType::MalformedApiToken) => {
					error!(
						"API token rejected ({}: {}). Run `patr -w {workspace_id} runner setup \
						 reconnect --runner-id {runner_id}` on this host, then restart the runner \
						 service.",
						err.status_code, err.body.message
					);
				}
				(_, ErrorType::RunnerAlreadyConnected) => {
					error!(
						"Runner already connected: the API still holds an earlier connection. If \
						 the runner stopped without closing it, that expires within two minutes and \
						 the runner gets back in on its own. If it lasts longer, another machine is \
						 running this runner: stop it there, or run `patr -w {workspace_id} runner \
						 setup reconnect --runner-id {runner_id}` here and restart the runner \
						 service."
					);
				}
				_ => error!("Failed to connect to the server: {:?}", err),
			}
			return Err(RunnerError::UpstreamServerError(err.body.error));
		}
		Err(_) => {
			error!("Failed to connect to the server: timed out after {CONNECT_TIMEOUT:?}");
			return Err(RunnerError::host("timed out connecting to the server"));
		}
	};
	info!("Connected to the server");

	if let Err(err) = stream
		.send(StreamRunnerDataForWorkspaceClientMsg::Handshake {
			// CI bakes the channel's full version (e.g. `0.18.0-alpha.1`) into
			// PATR_BUILD_VERSION; Cargo.toml only ever holds the bare release.
			version: option_env!("PATR_BUILD_VERSION")
				.unwrap_or(env!("CARGO_PKG_VERSION"))
				.parse()
				.expect("PATR_BUILD_VERSION must be a valid semver"),
			exposure_type: E::runner_exposure_type(config),
		})
		.await
	{
		error!("Failed to send the handshake: {:?}", err);
		return Err(RunnerError::host(format!(
			"failed to send the handshake: {err:?}"
		)));
	}

	Ok(stream)
}

/// Sync the local database with the upstream APIs. This function will listen
/// for messages from the server on the given connection, reconnecting whenever
/// it drops. It will notify the controller to reconcile the resources that are
/// changed on the server, and send their observed statuses back. This function
/// will exit if the exit signal is received.
pub async fn run<E: RunnerExecutor>(
	connection: Result<
		impl Stream<Item = Result<StreamRunnerDataForWorkspaceServerMsg, ErrorType>>
		+ Sink<StreamRunnerDataForWorkspaceClientMsg, Error: Debug>
		+ Unpin,
		RunnerError,
	>,
	mut receiver: mpsc::UnboundedReceiver<WebSocketMessage>,
	controller_tx: mpsc::UnboundedSender<ControllerMessage>,
	executor: E,
	config: RunnerSettings<E::Settings>,
	database: sqlx::Pool<DatabaseType>,
) -> Result<!, RunnerError> {
	// How long to wait before connecting again after a failed connect, doubling
	// up to a minute. Reset once connected.
	let mut reconnect_backoff = Duration::from_secs(1);

	info!("Syncing local database with upstream APIs");

	// The given connection was resynced by whoever opened it, so its first
	// resync waits for the timer
	if let Ok(stream) = connection {
		serve(
			stream,
			FULL_RESYNC_INTERVAL,
			&mut receiver,
			&controller_tx,
			&executor,
			&config,
			&database,
		)
		.await?;
	}

	// Connect to the server infinitely until the exit signal is received
	loop {
		let Ok(stream) = connect::<E>(&config).with_cancel_check().await? else {
			// Retry after the backoff, but break if the exit signal is received
			error!("Retrying in {reconnect_backoff:?}");
			time::sleep(reconnect_backoff).with_cancel_check().await?;
			reconnect_backoff = (reconnect_backoff * 2).min(Duration::from_secs(60));
			continue;
		};
		reconnect_backoff = Duration::from_secs(1);

		// Every connection opened here resyncs right away
		serve(
			stream,
			Duration::from_secs(0),
			&mut receiver,
			&controller_tx,
			&executor,
			&config,
			&database,
		)
		.await?;
	}
}

/// Serve one connection until it's lost: handle the server's messages, send
/// the observed statuses, and resync after `first_resync` and then on a timer.
/// Returns once it's time to reconnect, and only errors on the exit signal.
async fn serve<E: RunnerExecutor>(
	mut stream: impl Stream<Item = Result<StreamRunnerDataForWorkspaceServerMsg, ErrorType>>
	+ Sink<StreamRunnerDataForWorkspaceClientMsg, Error: Debug>
	+ Unpin,
	first_resync: Duration,
	receiver: &mut mpsc::UnboundedReceiver<WebSocketMessage>,
	controller_tx: &mpsc::UnboundedSender<ControllerMessage>,
	executor: &E,
	config: &RunnerSettings<E::Settings>,
	database: &sqlx::Pool<DatabaseType>,
) -> Result<(), RunnerError> {
	let RunnerSettings {
		workspace_id,
		runner_id,
		api_token,
		user_agent,
		..
	} = config.clone();

	// Clear any queued messages in the receiver to avoid processing stale updates
	// Every status is sent again after the full resync with the server
	let queued_messages = receiver.len();
	receiver.recv_many(&mut vec![], queued_messages).await;

	let mut pinned_sleeper = Box::pin(time::sleep(first_resync));

	// The API can regenerate the runner's tunnel token, so it's fetched again
	// on every connect.
	if E::runner_exposure_type(config).requires_tunnel() {
		match client::make_request(
			ApiRequest::<GetIngressTokenForRunnerRequest>::builder()
				.path(GetIngressTokenForRunnerPath {
					workspace_id,
					runner_id,
				})
				.headers(GetIngressTokenForRunnerRequestHeaders {
					authorization: api_token.clone(),
					user_agent: user_agent.clone(),
				})
				.build(),
		)
		.with_cancel_check()
		.await?
		{
			Ok(response) => match executor.update_ingress_token(response.body.token).await {
				Ok(true) => {
					let _ = controller_tx
						.send(ControllerMessage::Enqueue(BTreeSet::from([Owner::System])));
				}
				Ok(false) => {}
				Err(err) => error!(?err, "Failed to store the ingress token"),
			},
			Err(err) => warn!(error = ?err.body.error, "Failed to fetch the ingress token"),
		}
	}

	trace!("Syncing all resources before starting streaming");
	'message: loop {
		let Some(stream_message) = future::select(
			&mut pinned_sleeper,
			future::select(stream.next(), Box::pin(receiver.recv())),
		)
		.with_cancel_check()
		.await?
		.into_right() else {
			// Every 15 minutes, resync all resources to make sure everything is fine
			info!("Resyncing all resources with upstream to make sure everything is fine");
			pinned_sleeper = Box::pin(time::sleep(FULL_RESYNC_INTERVAL));

			// A failure (rate-limit, transient upstream error, network blip)
			// leaves the local database untouched and the connection up. The
			// next resync tries again.
			if let Err(err) = handle_full_resync(config, database)
				.with_cancel_check()
				.await?
			{
				error!("Failed to sync all resources: {:?}", err);
				continue 'message;
			}
			info!("All resources synced successfully");

			let _ = controller_tx.send(ControllerMessage::ReconcileAll);

			let Ok(()) = send_observed_statuses(database, &mut stream)
				.await
				.inspect_err(|err| {
					error!("Failed to send client message: {:?}", err);
					error!("Retrying connection to server");
				})
			else {
				// Retry after 5 seconds, but break if the exit signal is received
				time::sleep(Duration::from_secs(5))
					.with_cancel_check()
					.await?;
				return Ok(());
			};

			continue 'message;
		};

		let reconcile_message = match stream_message {
			Either::Left((reconcile_message, _)) => reconcile_message,
			Either::Right((executor_message, _)) => {
				let Some(executor_message) = executor_message else {
					// The executor message channel has been closed. This should not happen
					error!("Executor message channel has been closed");
					continue 'message;
				};
				let client_msg = match executor_message {
					WebSocketMessage::SendStatusUpdate {
						resource_id,
						status,
					} => StreamRunnerDataForWorkspaceClientMsg::DeploymentStatusUpdated {
						id: resource_id,
						status,
					},
				};
				let Ok(()) = stream.send(client_msg).await.inspect_err(|err| {
					error!("Failed to send client message: {:?}", err);
					error!("Retrying connection to server");
				}) else {
					// Retry after 5 seconds, but break if the exit signal is received
					time::sleep(Duration::from_secs(5))
						.with_cancel_check()
						.await?;
					return Ok(());
				};

				continue 'message;
			}
		};

		match reconcile_message {
			Some(Ok(response)) => {
				if let Err(err) = handle_server_message(config, database, controller_tx, response)
					.with_cancel_check()
					.await?
				{
					// Failed to handle the message. The resync after reconnecting
					// catches it up
					error!("Failed to handle the message: {err}");
					error!("Restarting connection to server");
					// Retry after 1 second, but break if the exit signal is received
					time::sleep(Duration::from_secs(1))
						.with_cancel_check()
						.await?;
					return Ok(());
				}
			}
			Some(Err(err)) => {
				// Data from the websocket failed
				error!("Failed to connect to the server: {:?}", err);
				error!("Retrying in 1 second");

				// Retry after 1 second, but break if the exit signal is received
				time::sleep(Duration::from_secs(1))
					.with_cancel_check()
					.await?;

				return Ok(());
			}
			None => {
				// Websocket disconnected. Reconnect
				error!("Connection to server closed");
				error!("Retrying in 2 seconds");
				// Retry after 2 seconds, but break if the exit signal is received
				time::sleep(Duration::from_secs(2))
					.with_cancel_check()
					.await?;

				return Ok(());
			}
		}
	}
}

/// Handle a message received from the upstream server.
async fn handle_server_message<S: Clone>(
	config: &RunnerSettings<S>,
	database: &sqlx::Pool<DatabaseType>,
	controller_tx: &mpsc::UnboundedSender<ControllerMessage>,
	msg: StreamRunnerDataForWorkspaceServerMsg,
) -> Result<(), RunnerError> {
	use StreamRunnerDataForWorkspaceServerMsg::*;

	// We need workspace_id + auth for domain resolution on managed URL events.
	let RunnerSettings {
		workspace_id,
		runner_id,
		api_token,
		user_agent,
		..
	} = config.clone();

	match msg {
		DeploymentUpdated {
			deployment,
			running_details,
		} => {
			if deployment.runner != runner_id {
				// A deployment that moved off this runner arrives as a
				// `DeploymentDeleted`, so this one was never ours.
				warn!(
					deployment_id = %deployment.id,
					"Received a deployment for another runner — ignoring"
				);
				return Ok(());
			}
			let deployment_id = deployment.id;
			let mut transaction = database.begin().await?;
			db_helpers::upsert_deployment_in_database(
				&mut transaction,
				deployment,
				running_details,
			)
			.await?;
			transaction.commit().await?;
			// Its managed URLs route to it, so they follow its changes.
			let managed_url_ids = query!(
				r#"
				SELECT
					id
				FROM
					managed_url
				WHERE
					deployment_id = $1;
				"#,
				deployment_id
			)
			.fetch_all(database)
			.await?
			.into_iter()
			.map(|row| Owner::ManagedUrl(row.id))
			.collect::<BTreeSet<_>>();
			let _ = controller_tx.send(ControllerMessage::Enqueue(
				managed_url_ids
					.into_iter()
					.chain([Owner::Deployment(deployment_id)])
					.collect(),
			));
		}
		DeploymentDeleted { id } => {
			// The deployment's managed URLs go with it, so read them first to
			// tell their owners too.
			let managed_url_ids = query!(
				r#"
				SELECT
					id
				FROM
					managed_url
				WHERE
					deployment_id = $1;
				"#,
				id
			)
			.fetch_all(database)
			.await?
			.into_iter()
			.map(|row| row.id)
			.collect::<Vec<_>>();

			let mut transaction = database.begin().await?;
			db_helpers::delete_deployment_in_database(&mut transaction, id).await?;
			transaction.commit().await?;
			let _ = controller_tx.send(ControllerMessage::Enqueue(
				managed_url_ids
					.into_iter()
					.map(Owner::ManagedUrl)
					.chain([Owner::Deployment(id)])
					.collect(),
			));
		}
		ManagedUrlUpdated { managed_url } => {
			let ManagedUrlType::ProxyDeployment {
				deployment_id,
				port,
			} = managed_url.url_type
			else {
				// Server should only stream ProxyDeployment URLs to the runner;
				// drop anything else defensively.
				warn!(
					id = %managed_url.id,
					"Received non-ProxyDeployment managed URL on stream — ignoring"
				);
				return Ok(());
			};
			let domain = client::make_request(
				ApiRequest::<GetDomainInfoInWorkspaceRequest>::builder()
					.path(GetDomainInfoInWorkspacePath {
						workspace_id,
						domain_id: managed_url.domain_id,
					})
					.query(())
					.headers(GetDomainInfoInWorkspaceRequestHeaders {
						authorization: api_token.clone(),
						user_agent: user_agent.clone(),
					})
					.body(GetDomainInfoInWorkspaceRequest)
					.build(),
			)
			.await
			.map_err(|err| RunnerError::UpstreamServerError(err.body.error))?
			.body
			.workspace_domain
			.data
			.name;
			let host = if managed_url.sub_domain == "@" {
				domain
			} else {
				format!("{}.{}", managed_url.sub_domain, domain)
			};
			let mut transaction = database.begin().await?;
			db_helpers::upsert_managed_url_in_database(
				&mut transaction,
				managed_url.id,
				&host,
				&managed_url.path,
				deployment_id,
				port,
			)
			.await?;
			transaction.commit().await?;
			let _ = controller_tx.send(ControllerMessage::Enqueue(BTreeSet::from([
				Owner::ManagedUrl(managed_url.id),
			])));
		}
		ManagedUrlDeleted { id } => {
			let mut transaction = database.begin().await?;
			db_helpers::delete_managed_url_in_database(&mut transaction, id).await?;
			transaction.commit().await?;
			let _ = controller_tx.send(ControllerMessage::Enqueue(BTreeSet::from([
				Owner::ManagedUrl(id),
			])));
		}
		SecretUpdated { id, last_updated } => {
			let mut transaction = database.begin().await?;
			db_helpers::upsert_secret_in_database(&mut transaction, id, last_updated).await?;
			let deployment_ids = query!(
				r#"
				SELECT DISTINCT
					deployment_id
				FROM
					deployment_environment_variable
				WHERE
					secret_id = $1;
				"#,
				id
			)
			.fetch_all(&mut *transaction)
			.await?
			.into_iter()
			.map(|row| row.deployment_id)
			.collect::<Vec<_>>();
			transaction.commit().await?;
			// The secret's version is part of each deployment's spec, so a
			// rotation re-applies the deployments using it.
			let _ = controller_tx.send(ControllerMessage::Enqueue(
				deployment_ids.into_iter().map(Owner::Deployment).collect(),
			));
		}
		HandshakeRequired => {
			warn!("Server requested handshake to be sent again");
		}
	}

	Ok(())
}

/// Sends every observed status upstream, bringing the API's copy up to date
/// with whatever it missed. Only a failed send is returned, since it means the
/// connection is gone.
pub async fn send_observed_statuses<S>(
	database: &sqlx::Pool<DatabaseType>,
	sink: &mut S,
) -> Result<(), S::Error>
where
	S: Sink<StreamRunnerDataForWorkspaceClientMsg> + Unpin,
{
	let Ok(rows) = query!(
		r#"
		SELECT
			id,
			observed_status AS "observed_status!"
		FROM
			deployment
		WHERE
			observed_status IS NOT NULL;
		"#
	)
	.fetch_all(database)
	.await
	.inspect_err(|err| error!("Failed to load the observed statuses: {:?}", err)) else {
		return Ok(());
	};
	for row in rows {
		sink.send(
			StreamRunnerDataForWorkspaceClientMsg::DeploymentStatusUpdated {
				id: row.id,
				status: row.observed_status,
			},
		)
		.await?;
	}
	Ok(())
}

/// Handle the periodic full resync: re-fetch this runner's deployments, their
/// managed URLs and the workspace's secrets from the upstream API, then write
/// only what differs from SQLite.
pub async fn handle_full_resync<S: Clone>(
	config: &RunnerSettings<S>,
	database: &sqlx::Pool<DatabaseType>,
) -> Result<(), RunnerError> {
	let RunnerSettings {
		workspace_id,
		runner_id,
		api_token,
		user_agent,
		..
	} = config.clone();

	info!("Starting full resync with upstream API");

	// Everything is fetched before anything is written. Any failed call (a 429,
	// a 5xx, a dropped connection) aborts the resync with SQLite untouched,
	// instead of reading as resources that were deleted.
	let mut deployments = BTreeMap::<Uuid, (Deployment, DeploymentRunningDetails)>::new();
	let mut page = 0usize;

	loop {
		let response = client::make_request(
			ApiRequest::<ListDeploymentRequest>::builder()
				.path(ListDeploymentPath { workspace_id })
				.query(ListResourceQuery {
					sort: Default::default(),
					search: DeploymentSearchParams {
						runner: Some(ResourceSearcher {
							resource_id: runner_id,
						}),
						..Default::default()
					},
					count: ListResourceQuery::DEFAULT_PAGE_SIZE,
					page,
					additional_query: (),
				})
				.headers(ListDeploymentRequestHeaders {
					authorization: api_token.clone(),
					user_agent: user_agent.clone(),
				})
				.body(ListDeploymentRequest)
				.build(),
		)
		.await
		.map_err(|err| RunnerError::UpstreamServerError(err.body.error))?;
		let total_count = response.headers.total_count.0 as usize;

		for deployment in response.body.deployments {
			let info = client::make_request(
				ApiRequest::<GetDeploymentInfoRequest>::builder()
					.path(GetDeploymentInfoPath {
						workspace_id,
						deployment_id: deployment.id,
					})
					.query(())
					.headers(GetDeploymentInfoRequestHeaders {
						authorization: api_token.clone(),
						user_agent: user_agent.clone(),
					})
					.body(GetDeploymentInfoRequest)
					.build(),
			)
			.await
			.map_err(|err| RunnerError::UpstreamServerError(err.body.error))?
			.body;

			// It may have moved to another runner since it was listed.
			if info.deployment.runner != runner_id {
				continue;
			}
			deployments.insert(
				info.deployment.id,
				(info.deployment.data, info.running_details),
			);
		}

		if (page + 1) * ListResourceQuery::DEFAULT_PAGE_SIZE >= total_count {
			break;
		}
		page += 1;
	}

	// Paginate through all managed URLs in the workspace, filter to
	// ProxyDeployment URLs targeting our deployments, and resolve each domain
	// (cached).
	let mut managed_urls = BTreeMap::<Uuid, (String, String, Uuid, u16)>::new();
	let mut managed_url_page: usize = 0;
	let mut domain_cache: BTreeMap<Uuid, String> = BTreeMap::new();

	loop {
		let response = client::make_request(
			ApiRequest::<ListManagedURLRequest>::builder()
				.path(ListManagedURLPath { workspace_id })
				.query(ListResourceQuery {
					sort: Default::default(),
					search: Default::default(),
					count: ListResourceQuery::DEFAULT_PAGE_SIZE,
					page: managed_url_page,
					additional_query: (),
				})
				.headers(ListManagedURLRequestHeaders {
					authorization: api_token.clone(),
					user_agent: user_agent.clone(),
				})
				.body(ListManagedURLRequest)
				.build(),
		)
		.await
		.map_err(|err| RunnerError::UpstreamServerError(err.body.error))?;

		for url in &response.body.urls {
			let ManagedUrlType::ProxyDeployment {
				deployment_id,
				port,
			} = url.url_type
			else {
				continue;
			};
			if !deployments.contains_key(&deployment_id) {
				continue;
			}

			let domain = match domain_cache.get(&url.domain_id) {
				Some(d) => d.clone(),
				None => {
					let resolved = client::make_request(
						ApiRequest::<GetDomainInfoInWorkspaceRequest>::builder()
							.path(GetDomainInfoInWorkspacePath {
								workspace_id,
								domain_id: url.domain_id,
							})
							.query(())
							.headers(GetDomainInfoInWorkspaceRequestHeaders {
								authorization: api_token.clone(),
								user_agent: user_agent.clone(),
							})
							.body(GetDomainInfoInWorkspaceRequest)
							.build(),
					)
					.await
					.map_err(|err| RunnerError::UpstreamServerError(err.body.error))?
					.body
					.workspace_domain
					.data
					.name;
					domain_cache.insert(url.domain_id, resolved.clone());
					resolved
				}
			};

			let host = if url.sub_domain == "@" {
				domain
			} else {
				format!("{}.{}", url.sub_domain, domain)
			};
			managed_urls.insert(url.id, (host, url.path.clone(), deployment_id, port));
		}

		if (managed_url_page + 1) * ListResourceQuery::DEFAULT_PAGE_SIZE >=
			response.headers.total_count.0 as usize
		{
			break;
		}
		managed_url_page += 1;
	}

	// When each secret last changed, so a rotation missed while disconnected
	// still re-applies the deployments using it.
	let mut secrets = BTreeMap::<Uuid, OffsetDateTime>::new();
	let mut secret_page: usize = 0;

	loop {
		let response = client::make_request(
			ApiRequest::<ListSecretsForWorkspaceRequest>::builder()
				.path(ListSecretsForWorkspacePath { workspace_id })
				.query(ListResourceQuery {
					sort: Default::default(),
					search: Default::default(),
					count: ListResourceQuery::DEFAULT_PAGE_SIZE,
					page: secret_page,
					additional_query: (),
				})
				.headers(ListSecretsForWorkspaceRequestHeaders {
					authorization: api_token.clone(),
					user_agent: user_agent.clone(),
				})
				.body(ListSecretsForWorkspaceRequest)
				.build(),
		)
		.await
		.map_err(|err| RunnerError::UpstreamServerError(err.body.error))?;
		let total_count = response.headers.total_count.0 as usize;

		for secret in response.body.secrets {
			secrets.insert(secret.id, secret.data.last_updated);
		}

		if (secret_page + 1) * ListResourceQuery::DEFAULT_PAGE_SIZE >= total_count {
			break;
		}
		secret_page += 1;
	}

	// Only this loop writes these tables, so reading them outside the
	// transaction below can't miss a write.
	let local_deployment_ids = query!(
		r#"
		SELECT
			id
		FROM
			deployment;
		"#
	)
	.fetch_all(database)
	.await?
	.into_iter()
	.map(|row| row.id)
	.collect::<BTreeSet<_>>();

	let mut changed_deployment_ids = BTreeSet::new();
	for (deployment_id, (deployment, running_details)) in &deployments {
		// The public status the API derives and the runner aren't stored locally
		// (only `observed_status` is), so they're set to the local placeholders.
		let mut stored = deployment.clone();
		stored.status = DeploymentStatus::Observed(ObservedDeploymentStatus::Unknown);
		stored.runner = Uuid::nil();

		let unchanged = match get_local_deployment_info(database, *deployment_id).await {
			Ok((local, local_running_details)) => {
				local == stored && local_running_details == *running_details
			}
			Err(RunnerError::UpstreamServerError(ErrorType::ResourceDoesNotExist)) => false,
			Err(err) => return Err(err),
		};
		if !unchanged {
			changed_deployment_ids.insert(*deployment_id);
		}
	}

	let local_managed_urls = query!(
		r#"
		SELECT
			id,
			host,
			path,
			deployment_id,
			port
		FROM
			managed_url;
		"#
	)
	.fetch_all(database)
	.await?
	.into_iter()
	.map(|row| (row.id, (row.host, row.path, row.deployment_id, row.port)))
	.collect::<BTreeMap<_, _>>();

	let local_secrets = query!(
		r#"
		SELECT
			id,
			last_updated
		FROM
			secret;
		"#
	)
	.fetch_all(database)
	.await?
	.into_iter()
	.map(|row| (row.id, row.last_updated))
	.collect::<BTreeMap<_, _>>();

	let removed_deployment_ids = local_deployment_ids
		.into_iter()
		.filter(|deployment_id| !deployments.contains_key(deployment_id))
		.collect::<Vec<_>>();

	let mut transaction = database.begin().await?;

	// Removed deployments go first: their managed URLs go with them, and a
	// managed URL that moved to another deployment is written back below.
	for deployment_id in &removed_deployment_ids {
		db_helpers::delete_deployment_in_database(&mut transaction, *deployment_id).await?;
	}

	for (deployment_id, (deployment, running_details)) in deployments
		.into_iter()
		.filter(|(deployment_id, _)| changed_deployment_ids.contains(deployment_id))
	{
		db_helpers::upsert_deployment_in_database(
			&mut transaction,
			WithId::new(deployment_id, deployment),
			running_details,
		)
		.await?;
	}

	for managed_url_id in local_managed_urls
		.keys()
		.filter(|managed_url_id| !managed_urls.contains_key(managed_url_id))
	{
		db_helpers::delete_managed_url_in_database(&mut transaction, *managed_url_id).await?;
	}

	for (managed_url_id, managed_url) in &managed_urls {
		if local_managed_urls.get(managed_url_id) == Some(managed_url) {
			continue;
		}
		let (host, path, deployment_id, port) = managed_url;
		db_helpers::upsert_managed_url_in_database(
			&mut transaction,
			*managed_url_id,
			host,
			path,
			*deployment_id,
			*port,
		)
		.await?;
	}

	for secret_id in local_secrets
		.keys()
		.filter(|secret_id| !secrets.contains_key(secret_id))
	{
		query!(
			r#"
			DELETE FROM
				secret
			WHERE
				id = $1;
			"#,
			secret_id
		)
		.execute(&mut *transaction)
		.await?;
	}

	for (secret_id, last_updated) in &secrets {
		if local_secrets.get(secret_id) == Some(last_updated) {
			continue;
		}
		db_helpers::upsert_secret_in_database(&mut transaction, *secret_id, *last_updated).await?;
	}

	transaction.commit().await?;

	info!(
		removed = removed_deployment_ids.len(),
		changed = changed_deployment_ids.len(),
		"Full resync complete, triggering reconciliation"
	);

	Ok(())
}
