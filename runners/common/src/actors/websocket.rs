use std::{
	collections::{BTreeMap, BTreeSet},
	marker::PhantomData,
	pin::Pin,
	time::Duration,
};

use futures::{Sink, SinkExt, StreamExt};
use models::api::workspace::{
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
};
use ractor::{Actor, ActorProcessingErr, ActorRef, SupervisionEvent};
use ractor_actors::streams::spawn_stream_pump;

use super::{db_helpers, resource_supervisor::ResourceSupervisorMessage};
use crate::prelude::*;

/// Type-erased WS sink. The concrete type from `stream_request` is opaque
/// (`impl Sink`), so we box it with a normalized `String` error.
type BoxedWsSink = Pin<Box<dyn Sink<StreamRunnerDataForWorkspaceClientMsg, Error = String> + Send>>;

/// Messages for the [`WebSocketActor`].
///
/// The WebSocketActor manages the bidirectional WebSocket connection to the
/// upstream Patr API (managed mode only). It writes server-pushed changes to
/// SQLite and forwards status updates from resource actors back upstream.
#[derive(Debug)]
pub enum WebSocketMessage {
	/// A resource actor reports a status change to send upstream.
	SendStatusUpdate {
		/// The UUID of the resource whose status changed.
		resource_id: Uuid,
		/// The new status to report upstream.
		status: DeploymentStatus,
	},
	/// Attempt to (re)connect to the upstream WebSocket.
	Connect,
	/// A message received from the upstream server.
	ServerMessage(Box<StreamRunnerDataForWorkspaceServerMsg>),
	/// The WS read stream ended or errored — trigger reconnection.
	StreamEnded,
	/// Periodic full resync with the upstream API.
	FullResync,
}

/// Returns the interval between periodic full resyncs with the upstream API.
/// 30 seconds in debug, 15 mins in release.
fn full_resync_interval() -> Duration {
	if cfg!(debug_assertions) {
		Duration::from_secs(30)
	} else {
		Duration::from_secs(60 * 15)
	}
}

/// Arguments passed to [`WebSocketActor::pre_start`] to construct the initial
/// state.
pub struct WebSocketActorArgs<E: RunnerExecutor> {
	/// Runner configuration.
	pub config: RunnerSettings<E::Settings>,
	/// Database connection pool for SQLite access.
	pub database: sqlx::Pool<DatabaseType>,
	/// Reference to the ResourceSupervisor for sending resource notifications.
	pub supervisor_ref: ActorRef<ResourceSupervisorMessage>,
}

/// The mutable state held by a running [`WebSocketActor`].
pub struct WebSocketActorState<E: RunnerExecutor> {
	/// Runner configuration.
	pub config: RunnerSettings<E::Settings>,
	/// Database connection pool for SQLite access.
	pub database: sqlx::Pool<DatabaseType>,
	/// Reference to the ResourceSupervisor for sending resource notifications.
	pub supervisor_ref: ActorRef<ResourceSupervisorMessage>,
	/// Current reconnection backoff duration. Reset on successful connect.
	pub reconnect_backoff: Duration,
	/// Write half of the active WS connection. `None` when disconnected.
	pub ws_sink: Option<BoxedWsSink>,
}

/// Actor responsible for managing the WebSocket connection to the upstream
/// Patr API. Only active in managed mode.
///
/// Receives server-pushed resource changes, writes them to SQLite, and
/// notifies the
/// [`ResourceSupervisor`][super::resource_supervisor::ResourceSupervisor]. Also
/// forwards status updates from resource actors back upstream.
pub struct WebSocketActor<E: RunnerExecutor> {
	/// Marker for the executor generic.
	_phantom: PhantomData<E>,
}

impl<E: RunnerExecutor> WebSocketActor<E> {
	/// Creates a new [`WebSocketActor`] instance.
	pub fn new() -> Self {
		Self {
			_phantom: PhantomData,
		}
	}
}

impl<E> Actor for WebSocketActor<E>
where
	E: RunnerExecutor + Send + Sync + 'static,
{
	type Arguments = WebSocketActorArgs<E>;
	type Msg = WebSocketMessage;
	type State = WebSocketActorState<E>;

	async fn pre_start(
		&self,
		myself: ActorRef<Self::Msg>,
		args: Self::Arguments,
	) -> Result<Self::State, ActorProcessingErr> {
		// Trigger initial connection.
		let _ = myself.send_message(WebSocketMessage::Connect);

		Ok(WebSocketActorState {
			config: args.config,
			database: args.database,
			supervisor_ref: args.supervisor_ref,
			reconnect_backoff: Duration::from_secs(1),
			ws_sink: None,
		})
	}

	async fn handle(
		&self,
		myself: ActorRef<Self::Msg>,
		message: Self::Msg,
		state: &mut Self::State,
	) -> Result<(), ActorProcessingErr> {
		match message {
			WebSocketMessage::Connect => {
				handle_connect(myself, state).await;
			}
			WebSocketMessage::ServerMessage(msg) => {
				if let Err(err) = handle_server_message(state, *msg).await {
					// Without this log the supervisor only sees the bare WARN
					// "Supervised child stopped" — the underlying cause (which
					// stream-message arm failed and why) would be invisible.
					error!(?err, "Failed to handle server message; actor will restart");
					return Err(err);
				}
			}
			WebSocketMessage::StreamEnded => {
				state.ws_sink = None;
				warn!("WebSocket stream ended, scheduling reconnect");
				schedule_reconnect(&myself, state);
			}
			WebSocketMessage::SendStatusUpdate {
				resource_id,
				status,
			} => {
				if let Some(sink) = &mut state.ws_sink {
					let msg = StreamRunnerDataForWorkspaceClientMsg::DeploymentStatusUpdated {
						id: resource_id,
						status,
					};
					if sink.send(msg).await.is_err() {
						// Sink errored — connection is dead. Drop it, requeue
						// the message so it's retried after reconnect, and
						// wait for StreamEnded to trigger reconnect.
						state.ws_sink = None;
						let _ = myself.send_message(WebSocketMessage::SendStatusUpdate {
							resource_id,
							status,
						});
					}
				}
			}
			WebSocketMessage::FullResync => {
				// A resync failure (rate-limit, transient upstream error, network
				// blip) must NOT tear down the live WebSocket.
				//
				// The local DB is untouched on failure: handle_full_resync does all
				// its writes in a transaction that rolls back if it returns early.
				if let Err(err) = handle_full_resync(myself.clone(), state).await {
					error!(
						?err,
						"Full resync failed; keeping connection alive, will retry on timer"
					);
					// handle_full_resync only re-arms the periodic timer on success,
					// so re-arm it here to guarantee the next attempt is scheduled.
					myself.send_after(full_resync_interval(), || WebSocketMessage::FullResync);
				}
			}
		}
		Ok(())
	}

	/// Surface the stream pump's failure cause before stopping. ractor's
	/// default `handle_supervisor_evt` calls `myself.stop(None)` and discards
	/// the inner `ActorProcessingErr`, so the parent supervisor only sees a
	/// clean termination and the actual error (which lives on the child's
	/// `ActorFailed` event) is lost. Log it here so the next-level supervisor
	/// log has something to correlate against.
	async fn handle_supervisor_evt(
		&self,
		myself: ActorRef<Self::Msg>,
		message: SupervisionEvent,
		_state: &mut Self::State,
	) -> Result<(), ActorProcessingErr> {
		match &message {
			SupervisionEvent::ActorFailed(cell, err) => {
				error!(
					child_id = %cell.get_id(),
					child_name = ?cell.get_name(),
					?err,
					"WS actor child failed — propagating stop"
				);
				myself.stop(None);
			}
			SupervisionEvent::ActorTerminated(cell, _, reason) => {
				debug!(
					child_id = %cell.get_id(),
					child_name = ?cell.get_name(),
					?reason,
					"WS actor child terminated cleanly"
				);
				myself.stop(None);
			}
			_ => {}
		}
		Ok(())
	}
}

/// Attempt to establish the WebSocket connection. On success, set up a stream
/// pump for reading and an mpsc-bridged task for writing.
async fn handle_connect<E>(myself: ActorRef<WebSocketMessage>, state: &mut WebSocketActorState<E>)
where
	E: RunnerExecutor + Send + Sync + 'static,
{
	let RunnerMode::Managed {
		workspace_id,
		runner_id,
		api_token,
		user_agent,
	} = state.config.mode.clone()
	else {
		debug!("Runner is in self-hosted mode, WebSocketActor not needed");
		return;
	};

	// Drop any existing sink from a previous connection.
	state.ws_sink = None;

	info!("Connecting to upstream WebSocket");

	let stream = match client::stream_request(
		ApiRequest::<StreamRunnerDataForWorkspaceRequest>::builder()
			.path(StreamRunnerDataForWorkspacePath {
				workspace_id,
				runner_id,
			})
			.headers(StreamRunnerDataForWorkspaceRequestHeaders {
				authorization: api_token.clone(),
				user_agent: user_agent.clone(),
			})
			.build(),
	)
	.await
	{
		Ok(stream) => stream,
		Err(err) => {
			error!("Failed to connect to upstream WebSocket: {:?}", err);
			schedule_reconnect(&myself, state);
			return;
		}
	};

	info!("Connected to upstream WebSocket");
	state.reconnect_backoff = Duration::from_secs(1);

	// Split into read (Stream) and write (Sink) halves.
	let (mut sink, read_stream) = stream.split();

	// Send the runner's exposure type before pumping.
	if sink
		.send(
			StreamRunnerDataForWorkspaceClientMsg::SetRunnerExposureType {
				exposure_type: E::runner_exposure_type(&state.config),
			},
		)
		.await
		.is_err()
	{
		error!("Failed to send exposure type, reconnecting");
		schedule_reconnect(&myself, state);
		return;
	}

	// Set up the read side: spawn_stream_pump forwards each stream item
	// as a ServerMessage to this actor. On stream end (None), send
	// StreamEnded to trigger reconnection.
	if let Err(err) = spawn_stream_pump(
		read_stream,
		myself.clone(),
		|item| match item {
			Some(Ok(msg)) => WebSocketMessage::ServerMessage(Box::new(msg)),
			Some(Err(_)) | None => WebSocketMessage::StreamEnded,
		},
		None,
	)
	.await
	{
		error!("Failed to spawn stream pump: {:?}", err);
		schedule_reconnect(&myself, state);
		return;
	}

	// Store the write half, boxing it to erase the opaque type.
	state.ws_sink = Some(Box::pin(sink.sink_map_err(|e| format!("{e:?}"))));

	// Trigger an immediate full resync so a freshly-connected runner picks up
	// pre-existing deployments without waiting for the periodic timer — server
	// pushes only fire on state changes, not on connect. The handler re-arms
	// the periodic timer when it finishes.
	let _ = myself.send_message(WebSocketMessage::FullResync);
}

/// Schedule a `Connect` message with exponential backoff.
fn schedule_reconnect<E: RunnerExecutor>(
	myself: &ActorRef<WebSocketMessage>,
	state: &mut WebSocketActorState<E>,
) {
	let backoff = state.reconnect_backoff;
	info!(?backoff, "Scheduling reconnect");
	myself.send_after(backoff, || WebSocketMessage::Connect);
	state.reconnect_backoff = (backoff * 2).min(Duration::from_secs(60));
}

/// Handle a `ServerMessage` received from the stream pump.
async fn handle_server_message<E>(
	state: &mut WebSocketActorState<E>,
	msg: StreamRunnerDataForWorkspaceServerMsg,
) -> Result<(), ActorProcessingErr>
where
	E: RunnerExecutor + Send + Sync + 'static,
{
	use StreamRunnerDataForWorkspaceServerMsg::*;

	// We need workspace_id + auth for domain resolution on managed URL events.
	let RunnerMode::Managed {
		workspace_id,
		api_token,
		user_agent,
		..
	} = state.config.mode.clone()
	else {
		// Self-hosted runners shouldn't receive these events.
		return Ok(());
	};

	match msg {
		DeploymentCreated {
			deployment,
			running_details,
		} |
		DeploymentUpdated {
			deployment,
			running_details,
		} => {
			let deployment_id = deployment.id;
			let mut transaction = state.database.begin().await?;
			db_helpers::upsert_deployment_in_database(
				&mut transaction,
				deployment,
				running_details,
			)
			.await?;
			transaction.commit().await?;
			let _ = state
				.supervisor_ref
				.send_message(ResourceSupervisorMessage::UpsertResource {
					resource_id: deployment_id,
					resource_type: ResourceType::Deployment,
				});
		}
		DeploymentDeleted { id } => {
			let mut transaction = state.database.begin().await?;
			db_helpers::delete_deployment_in_database(&mut transaction, id).await?;
			transaction.commit().await?;
			let _ = state
				.supervisor_ref
				.send_message(ResourceSupervisorMessage::DeleteResource {
					resource_id: id,
					resource_type: ResourceType::Deployment,
				});
		}
		ManagedUrlCreated { managed_url } | ManagedUrlUpdated { managed_url } => {
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
			let mut transaction = state.database.begin().await?;
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
			let _ = state
				.supervisor_ref
				.send_message(ResourceSupervisorMessage::UpsertResource {
					resource_id: managed_url.id,
					resource_type: ResourceType::ManagedURL,
				});
		}
		ManagedUrlDeleted { id } => {
			let mut transaction = state.database.begin().await?;
			db_helpers::delete_managed_url_in_database(&mut transaction, id).await?;
			transaction.commit().await?;
			let _ = state
				.supervisor_ref
				.send_message(ResourceSupervisorMessage::DeleteResource {
					resource_id: id,
					resource_type: ResourceType::ManagedURL,
				});
		}
		ExposureTypeRequired => {
			warn!("Server requested exposure type to be set again");
		}
	}

	Ok(())
}

/// Handle the periodic `FullResync`: re-fetch all deployments from the
/// upstream API and reconcile with SQLite.
async fn handle_full_resync<E>(
	myself: ActorRef<WebSocketMessage>,
	state: &mut WebSocketActorState<E>,
) -> Result<(), ActorProcessingErr>
where
	E: RunnerExecutor + Send + Sync + 'static,
{
	let RunnerMode::Managed {
		workspace_id,
		runner_id,
		api_token,
		user_agent,
	} = state.config.mode.clone()
	else {
		return Ok(());
	};

	info!("Starting full resync with upstream API");

	let mut transaction = state.database.begin().await?;

	// Clear managed URLs first so the deployment-FK they reference is free
	// to be cleared next.
	db_helpers::delete_all_managed_urls_in_database(&mut transaction).await?;

	// Clear all deployment-related tables.
	query("DELETE FROM deployment_volume_mount;")
		.execute(&mut *transaction)
		.await?;
	query("DELETE FROM deployment_deploy_history;")
		.execute(&mut *transaction)
		.await?;
	query("DELETE FROM deployment_config_mounts;")
		.execute(&mut *transaction)
		.await?;
	query("DELETE FROM deployment_exposed_port;")
		.execute(&mut *transaction)
		.await?;
	query("DELETE FROM deployment_environment_variable;")
		.execute(&mut *transaction)
		.await?;
	query("DELETE FROM deployment;")
		.execute(&mut *transaction)
		.await?;

	// Paginate through all deployments from the upstream API.
	let mut page: usize = 0;

	loop {
		let response = match client::make_request(
			ApiRequest::<ListDeploymentRequest>::builder()
				.path(ListDeploymentPath { workspace_id })
				.query(ListResourceQuery {
					sort: Default::default(),
					search: Default::default(),
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
		{
			Ok(response) => response,
			// Paginating past the last page returns PageOutOfBounds — that's the
			// normal end-of-list signal during resync, not a failure.
			Err(err) if matches!(err.body.error, ErrorType::PageOutOfBounds) => break,
			Err(err) => return Err(RunnerError::UpstreamServerError(err.body.error).into()),
		};

		if page * ListResourceQuery::DEFAULT_PAGE_SIZE >= response.headers.total_count.0 as usize {
			break;
		}

		for deployment in response
			.body
			.deployments
			.into_iter()
			.filter(|d| d.runner == runner_id)
		{
			let deployment_id = deployment.id;

			let info = client::make_request(
				ApiRequest::<GetDeploymentInfoRequest>::builder()
					.path(GetDeploymentInfoPath {
						workspace_id,
						deployment_id,
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
			.map_err(|err| RunnerError::UpstreamServerError(err.body.error))?;

			db_helpers::upsert_deployment_in_database(
				&mut transaction,
				info.body.deployment,
				info.body.running_details,
			)
			.await?;
		}

		page += 1;
	}

	// Collect deployment IDs we just synced; managed URLs only get persisted
	// for deployments running on this runner. We need this before committing
	// so the SQLite read sees them, but we can also use the in-memory set.
	let local_deployment_ids = query(
		r#"
		SELECT
			id
		FROM
			deployment
		WHERE
			deleted IS NULL;
		"#,
	)
	.fetch_all(&mut *transaction)
	.await?
	.into_iter()
	.filter_map(|row| row.try_get::<Uuid, _>("id").ok())
	.collect::<BTreeSet<Uuid>>();

	// Paginate through all managed URLs in the workspace, filter to
	// ProxyDeployment URLs targeting our deployments, resolve each domain
	// (cached), and write into SQLite.
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
			if !local_deployment_ids.contains(&deployment_id) {
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
			db_helpers::upsert_managed_url_in_database(
				&mut transaction,
				url.id,
				&host,
				&url.path,
				deployment_id,
				port,
			)
			.await?;
		}

		if (managed_url_page + 1) * ListResourceQuery::DEFAULT_PAGE_SIZE >=
			response.headers.total_count.0 as usize
		{
			break;
		}
		managed_url_page += 1;
	}

	transaction.commit().await?;

	info!("Full resync complete, triggering reconciliation");

	let _ = state
		.supervisor_ref
		.send_message(ResourceSupervisorMessage::Reconcile);

	// Schedule the next full resync.
	myself.send_after(full_resync_interval(), || WebSocketMessage::FullResync);

	Ok(())
}
