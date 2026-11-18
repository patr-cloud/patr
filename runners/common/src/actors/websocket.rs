use std::{
	collections::{BTreeMap, BTreeSet},
	marker::PhantomData,
	pin::Pin,
	time::Duration,
};

use futures::{Sink, SinkExt, StreamExt};
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
use ractor::{Actor, ActorProcessingErr, ActorRef, SupervisionEvent};
use ractor_actors::streams::spawn_stream_pump;
use time::OffsetDateTime;

use super::{
	controller::{CONTROLLER_NAME, ControllerMessage},
	db_helpers::{self, get_local_deployment_info},
};
use crate::prelude::*;

/// The name the WebSocket actor registers under, so other actors can find it.
pub const WEBSOCKET_NAME: &str = "websocket";

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
	/// A resource actor reports a status change to send upstream. Dropped while
	/// disconnected, since every resync sends them all.
	SendStatusUpdate {
		/// The UUID of the resource whose status changed.
		resource_id: Uuid,
		/// The new status to report upstream.
		status: ObservedDeploymentStatus,
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
	/// The executor, which stores the ingress token.
	pub executor: E,
}

/// The mutable state held by a running [`WebSocketActor`].
pub struct WebSocketActorState<E: RunnerExecutor> {
	/// Runner configuration.
	pub config: RunnerSettings<E::Settings>,
	/// Database connection pool for SQLite access.
	pub database: sqlx::Pool<DatabaseType>,
	/// The executor, which stores the ingress token.
	pub executor: E,
	/// Current reconnection backoff duration. Reset on successful connect.
	pub reconnect_backoff: Duration,
	/// Write half of the active WS connection. `None` when disconnected.
	pub ws_sink: Option<BoxedWsSink>,
}

/// Actor responsible for managing the WebSocket connection to the upstream
/// Patr API. Only active in managed mode.
///
/// Receives server-pushed resource changes, writes them to SQLite, and
/// enqueues the owners they affect with the
/// [`Controller`][super::controller::Controller]. Also publishes observed
/// statuses back upstream.
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
			executor: args.executor,
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
						// Sink errored — connection is dead. Drop it and wait for
						// StreamEnded to trigger reconnect, whose resync sends
						// this status again.
						state.ws_sink = None;
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
	let RunnerSettings {
		workspace_id,
		runner_id,
		api_token,
		user_agent,
		..
	} = state.config.clone();

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
			// Keep retrying even when the fix is out of our hands: exiting would
			// only have systemd restart us into the same rejection.
			match (err.status_code, &err.body.error) {
				(StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN, _) |
				(_, ErrorType::MalformedApiToken) => {
					error!(
						"Upstream rejected the API token of runner {runner_id} in workspace \
						 {workspace_id} ({}: {}). It was probably regenerated, or the runner was \
						 deleted. Run `patr -w {workspace_id} runner setup reconnect --runner-id \
						 {runner_id}` on this host to write a new token to `apiToken` in the \
						 runner's config (it regenerates the token if the CLI is logged in; \
						 otherwise use Regenerate token on the runner's page in the dashboard and \
						 paste it), then restart the runner service.",
						err.status_code, err.body.message
					);
				}
				(_, ErrorType::RunnerAlreadyConnected) => {
					error!(
						"Upstream says runner {runner_id} is already connected. This clears \
						 within two minutes if the runner restarted abruptly. If it persists, \
						 another machine is running this runner: stop it there, or move the \
						 runner here with `patr -w {workspace_id} runner setup reconnect \
						 --runner-id {runner_id}`, which regenerates its token and disconnects \
						 the other machine, then restart the runner service."
					);
				}
				_ => error!("Failed to connect to upstream WebSocket: {:?}", err),
			}
			schedule_reconnect(&myself, state);
			return;
		}
	};

	info!("Connected to upstream WebSocket");
	state.reconnect_backoff = Duration::from_secs(1);

	// Split into read (Stream) and write (Sink) halves.
	let (mut sink, read_stream) = stream.split();

	// Send the handshake before pumping.
	if sink
		.send(StreamRunnerDataForWorkspaceClientMsg::Handshake {
			// CI bakes the channel's full version (e.g. `0.18.0-alpha.1`) into
			// PATR_BUILD_VERSION; Cargo.toml only ever holds the bare release.
			version: option_env!("PATR_BUILD_VERSION")
				.unwrap_or(env!("CARGO_PKG_VERSION"))
				.parse()
				.expect("PATR_BUILD_VERSION must be a valid semver"),
			exposure_type: E::runner_exposure_type(&state.config),
		})
		.await
		.is_err()
	{
		error!("Failed to send handshake, reconnecting");
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

	// The API can regenerate the runner's tunnel token, so it's fetched again
	// on every connect.
	if E::runner_exposure_type(&state.config).requires_tunnel() {
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
		.await
		{
			Ok(response) => match state
				.executor
				.update_ingress_token(response.body.token)
				.await
			{
				Ok(true) => enqueue([Owner::System]),
				Ok(false) => {}
				Err(err) => error!(?err, "Failed to store the ingress token"),
			},
			Err(err) => warn!(error = ?err.body.error, "Failed to fetch the ingress token"),
		}
	}

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

/// Tells the controller these owners may be out of date.
fn enqueue(owners: impl IntoIterator<Item = Owner>) {
	if let Some(controller) = ActorRef::<ControllerMessage>::where_is(CONTROLLER_NAME) {
		let _ = controller.send_message(ControllerMessage::Enqueue(owners.into_iter().collect()));
	}
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
	let RunnerSettings {
		workspace_id,
		runner_id,
		api_token,
		user_agent,
		..
	} = state.config.clone();

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
			let mut transaction = state.database.begin().await?;
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
			.fetch_all(&state.database)
			.await?
			.into_iter()
			.map(|row| Owner::ManagedUrl(row.id))
			.collect::<BTreeSet<_>>();
			enqueue(
				managed_url_ids
					.into_iter()
					.chain([Owner::Deployment(deployment_id)]),
			);
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
			.fetch_all(&state.database)
			.await?
			.into_iter()
			.map(|row| row.id)
			.collect::<Vec<_>>();

			let mut transaction = state.database.begin().await?;
			db_helpers::delete_deployment_in_database(&mut transaction, id).await?;
			transaction.commit().await?;
			enqueue(
				managed_url_ids
					.into_iter()
					.map(Owner::ManagedUrl)
					.chain([Owner::Deployment(id)]),
			);
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
			enqueue([Owner::ManagedUrl(managed_url.id)]);
		}
		ManagedUrlDeleted { id } => {
			let mut transaction = state.database.begin().await?;
			db_helpers::delete_managed_url_in_database(&mut transaction, id).await?;
			transaction.commit().await?;
			enqueue([Owner::ManagedUrl(id)]);
		}
		SecretUpdated { id, last_updated } => {
			let mut transaction = state.database.begin().await?;
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
			enqueue(deployment_ids.into_iter().map(Owner::Deployment));
		}
		HandshakeRequired => {
			warn!("Server requested handshake to be sent again");
		}
	}

	Ok(())
}

/// Handle the periodic `FullResync`: re-fetch this runner's deployments, their
/// managed URLs and the workspace's secrets from the upstream API, then write
/// only what differs from SQLite.
async fn handle_full_resync<E>(
	myself: ActorRef<WebSocketMessage>,
	state: &mut WebSocketActorState<E>,
) -> Result<(), ActorProcessingErr>
where
	E: RunnerExecutor + Send + Sync + 'static,
{
	let RunnerSettings {
		workspace_id,
		runner_id,
		api_token,
		user_agent,
		..
	} = state.config.clone();

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

	// Only this actor writes these tables, so reading them outside the
	// transaction below can't miss a write.
	let local_deployment_ids = query!(
		r#"
		SELECT
			id
		FROM
			deployment;
		"#
	)
	.fetch_all(&state.database)
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

		let unchanged = match get_local_deployment_info(&state.database, *deployment_id).await {
			Ok((local, local_running_details)) => {
				local == stored && local_running_details == *running_details
			}
			Err(RunnerError::UpstreamServerError(ErrorType::ResourceDoesNotExist)) => false,
			Err(err) => return Err(err.into()),
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
	.fetch_all(&state.database)
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
	.fetch_all(&state.database)
	.await?
	.into_iter()
	.map(|row| (row.id, row.last_updated))
	.collect::<BTreeMap<_, _>>();

	let removed_deployment_ids = local_deployment_ids
		.into_iter()
		.filter(|deployment_id| !deployments.contains_key(deployment_id))
		.collect::<Vec<_>>();

	let mut transaction = state.database.begin().await?;

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

	if let Some(controller) = ActorRef::<ControllerMessage>::where_is(CONTROLLER_NAME) {
		let _ = controller.send_message(ControllerMessage::Start);
		let _ = controller.send_message(ControllerMessage::Reconcile);
	}

	// Bring the API's copy of every observed status up to date too, catching up
	// whatever it missed. A failed send means the connection is gone, and the
	// next one's resync sends them all again.
	if let Some(sink) = &mut state.ws_sink {
		let rows = query!(
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
		.fetch_all(&state.database)
		.await?;

		for row in rows {
			if sink
				.send(
					StreamRunnerDataForWorkspaceClientMsg::DeploymentStatusUpdated {
						id: row.id,
						status: row.observed_status,
					},
				)
				.await
				.is_err()
			{
				// Sink errored — connection is dead. Drop it and wait for
				// StreamEnded to trigger reconnect.
				state.ws_sink = None;
				break;
			}
		}
	}

	// Schedule the next full resync.
	myself.send_after(full_resync_interval(), || WebSocketMessage::FullResync);

	Ok(())
}
