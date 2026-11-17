//! In-test server that impersonates the upstream Patr API for managed mode
//! tests. Handles both WebSocket streams (runner data) and REST endpoints
//! (ListDeployment, GetDeploymentInfo, ListManagedURL, GetDomainInfoInWorkspace,
//! ListSecretsForWorkspace) on port 3000.
//!
//! Each runner connects with a unique `runner_id` in the WS path, so
//! multiple tests can share the same server without interference.
//! [`ManagedServerState::set_rejected`] makes the stream turn a runner away
//! with a 401, the way the API does once the runner's token is regenerated.

use std::{
	collections::BTreeMap,
	sync::{Arc, Mutex},
};

use axum::{
	Json,
	Router,
	extract::{
		Path,
		Query,
		RawQuery,
		State,
		ws::{Message, WebSocket, WebSocketUpgrade},
	},
	response::{IntoResponse, Response},
	routing::get,
};
use axum_extra::TypedHeader;
use common::prelude::*;
use futures::{SinkExt, StreamExt};
use http::StatusCode;
use models::{
	ApiErrorResponseBody,
	api::workspace::{deployment::*, domain::*, managed_url::*, runner::*, secret::*},
	utils::False,
};
use tokio::sync::{OnceCell, broadcast};

/// Per-runner state for the test server. Each runner_id gets its own channel
/// pair for sending server messages and receiving client messages.
pub struct RunnerChannel {
	/// Send server messages (DeploymentUpdated, etc.) to this runner.
	pub server_tx: broadcast::Sender<String>,
	/// Client messages (Handshake, DeploymentStatusUpdated)
	/// received from this runner.
	pub client_msgs: Arc<Mutex<Vec<StreamRunnerDataForWorkspaceClientMsg>>>,
	/// Whether the stream rejects this runner's token with a 401.
	pub rejected: bool,
	/// How many connection attempts were rejected with a 401.
	pub rejected_attempts: usize,
}

/// Shared state for the test managed server.
pub struct ManagedServerState {
	/// Per-runner channels, keyed by runner_id.
	pub runners: Mutex<BTreeMap<Uuid, RunnerChannel>>,
	/// Deployments to return from ListDeployment/GetDeploymentInfo REST calls,
	/// keyed by deployment_id.
	pub deployments: Mutex<BTreeMap<Uuid, (WithId<Deployment>, DeploymentRunningDetails)>>,
	/// Secrets to return from the ListSecretsForWorkspace REST call, keyed by
	/// secret_id.
	pub secrets: Mutex<BTreeMap<Uuid, WithId<Secret>>>,
	/// Managed URLs to return from the ListManagedURL REST call, keyed by
	/// managed_url_id.
	pub managed_urls: Mutex<BTreeMap<Uuid, WithId<ManagedUrl>>>,
	/// Domain names to return from the GetDomainInfoInWorkspace REST call,
	/// keyed by domain_id.
	pub domains: Mutex<BTreeMap<Uuid, String>>,
	/// List calls that fail, keyed by `{workspace_id}/{list}` (list being
	/// "deployment", "managedUrl" or "secret"), with how many times each
	/// failure was served.
	pub list_errors: Mutex<BTreeMap<String, (StatusCode, usize)>>,
}

impl ManagedServerState {
	pub fn new() -> Arc<Self> {
		Arc::new(Self {
			runners: Mutex::new(BTreeMap::new()),
			deployments: Mutex::new(BTreeMap::new()),
			secrets: Mutex::new(BTreeMap::new()),
			managed_urls: Mutex::new(BTreeMap::new()),
			domains: Mutex::new(BTreeMap::new()),
			list_errors: Mutex::new(BTreeMap::new()),
		})
	}

	/// Register a runner and return its broadcast sender for sending server
	/// messages.
	pub fn register_runner(&self, runner_id: Uuid) -> broadcast::Sender<String> {
		let (tx, _) = broadcast::channel(64);
		self.runners.lock().unwrap().insert(
			runner_id,
			RunnerChannel {
				server_tx: tx.clone(),
				client_msgs: Arc::new(Mutex::new(Vec::new())),
				rejected: false,
				rejected_attempts: 0,
			},
		);
		tx
	}

	/// Reject (or accept again) a runner's token on the stream. Rejecting also
	/// drops the runner's live connection, as the API does when the token is
	/// regenerated.
	pub fn set_rejected(&self, runner_id: Uuid, rejected: bool) {
		let mut runners = self.runners.lock().unwrap();
		let channel = runners.get_mut(&runner_id).unwrap();
		channel.rejected = rejected;
		if rejected {
			// Dropping the only sender ends the connection's write task.
			channel.server_tx = broadcast::channel(64).0;
		}
	}

	/// How many of a runner's connection attempts were rejected with a 401.
	pub fn rejected_attempts(&self, runner_id: Uuid) -> usize {
		self.runners
			.lock()
			.unwrap()
			.get(&runner_id)
			.map(|ch| ch.rejected_attempts)
			.unwrap_or_default()
	}

	/// Send a server message to a specific runner's WS connection.
	pub fn send_to_runner(&self, runner_id: Uuid, msg: &StreamRunnerDataForWorkspaceServerMsg) {
		let json = serde_json::to_string(msg).unwrap();
		if let Some(channel) = self.runners.lock().unwrap().get(&runner_id) {
			let _ = channel.server_tx.send(json);
		}
	}

	/// Get client messages received from a runner.
	pub fn get_client_msgs(&self, runner_id: Uuid) -> Vec<StreamRunnerDataForWorkspaceClientMsg> {
		self.runners
			.lock()
			.unwrap()
			.get(&runner_id)
			.map(|ch| ch.client_msgs.lock().unwrap().clone())
			.unwrap_or_default()
	}

	/// Add a deployment that the REST list/get endpoints will return.
	pub fn add_deployment(
		&self,
		deployment: WithId<Deployment>,
		details: DeploymentRunningDetails,
	) {
		self.deployments
			.lock()
			.unwrap()
			.insert(deployment.id, (deployment, details));
	}

	/// Stop returning a deployment from the REST list/get endpoints.
	pub fn remove_deployment(&self, deployment_id: Uuid) {
		self.deployments.lock().unwrap().remove(&deployment_id);
	}

	/// Add (or replace) a secret that the REST list endpoint will return.
	pub fn add_secret(&self, secret: WithId<Secret>) {
		self.secrets.lock().unwrap().insert(secret.id, secret);
	}

	/// Stop returning a secret from the REST list endpoint.
	pub fn remove_secret(&self, secret_id: Uuid) {
		self.secrets.lock().unwrap().remove(&secret_id);
	}

	/// Add (or replace) a managed URL that the REST list endpoint will return,
	/// along with its domain's name.
	pub fn add_managed_url(&self, managed_url: WithId<ManagedUrl>, domain: &str) {
		self.domains
			.lock()
			.unwrap()
			.insert(managed_url.domain_id, domain.to_string());
		self.managed_urls
			.lock()
			.unwrap()
			.insert(managed_url.id, managed_url);
	}

	/// Make a list call fail with `status` for a workspace, until cleared.
	pub fn fail_list(&self, workspace_id: Uuid, list: &'static str, status: StatusCode) {
		self.list_errors
			.lock()
			.unwrap()
			.insert(format!("{workspace_id}/{list}"), (status, 0));
	}

	/// How many times a failing list call has been served.
	pub fn list_failures_served(&self, workspace_id: Uuid, list: &'static str) -> usize {
		self.list_errors
			.lock()
			.unwrap()
			.get(&format!("{workspace_id}/{list}"))
			.map(|(_, served)| *served)
			.unwrap_or_default()
	}

	/// Let every list call succeed for a workspace again.
	pub fn clear_list_errors(&self, workspace_id: Uuid) {
		self.list_errors
			.lock()
			.unwrap()
			.retain(|key, _| !key.starts_with(&format!("{workspace_id}/")));
	}

	/// The error response a failing list call returns, if it's set to fail.
	fn list_error(
		&self,
		workspace_id: Uuid,
		list: &'static str,
	) -> Option<axum::response::Response> {
		let mut list_errors = self.list_errors.lock().unwrap();
		let (status, served) = list_errors.get_mut(&format!("{workspace_id}/{list}"))?;
		*served += 1;
		let error = if *status == StatusCode::TOO_MANY_REQUESTS {
			ErrorType::RateLimitExceeded
		} else {
			ErrorType::InternalServerError
		};
		Some(
			(
				*status,
				Json(ApiErrorResponseBody {
					success: False,
					error,
					message: "Injected failure".to_string(),
				}),
			)
				.into_response(),
		)
	}

	/// Close a runner's WS connection. The runner reconnects, and resyncs on
	/// the new connection.
	pub fn disconnect_runner(&self, runner_id: Uuid) {
		if let Some(channel) = self.runners.lock().unwrap().get_mut(&runner_id) {
			// Dropping the only sender ends the connection's write task.
			channel.server_tx = broadcast::channel(64).0;
		}
	}
}

/// Get or start the managed test server on port 3000. The server is shared
/// across all managed mode tests in the same process. If the port is already
/// bound (by another test process from nextest), this will panic — managed
/// mode tests must run sequentially or in a single process.
///
/// Uses a static OnceCell so the server is only started once per process.
static MANAGED_SERVER: OnceCell<Arc<ManagedServerState>> = OnceCell::const_new();

pub async fn get_managed_server() -> Arc<ManagedServerState> {
	MANAGED_SERVER
		.get_or_init(|| async {
			let state = ManagedServerState::new();

			let app = Router::new()
				.route(
					"/workspace/{workspace_id}/runner/{runner_id}/stream",
					get(ws_handler),
				)
				.route(
					"/workspace/{workspace_id}/deployment",
					get(list_deployments_handler),
				)
				.route(
					"/workspace/{workspace_id}/deployment/{deployment_id}",
					get(get_deployment_info_handler),
				)
				.route(
					"/workspace/{workspace_id}/managed-url",
					get(list_managed_urls_handler),
				)
				.route(
					"/workspace/{workspace_id}/secret",
					get(list_secrets_handler),
				)
				.route(
					"/workspace/{workspace_id}/domain/{domain_id}",
					get(get_domain_info_handler),
				)
				.with_state(state.clone());

			let socket = tokio::net::TcpSocket::new_v4().unwrap();
			socket.set_reuseaddr(true).unwrap();
			socket.bind("127.0.0.1:3000".parse().unwrap()).expect(
				"failed to bind test managed server on port 3000 — is something else using it?",
			);
			let listener = socket.listen(128).unwrap();

			tokio::spawn(async move {
				axum::serve(listener, app).await.ok();
			});

			// Give the server a moment to start.
			tokio::time::sleep(std::time::Duration::from_millis(50)).await;

			state
		})
		.await
		.clone()
}

async fn ws_handler(
	Path((_workspace_id, runner_id)): Path<(String, String)>,
	State(state): State<Arc<ManagedServerState>>,
	ws: WebSocketUpgrade,
) -> Response {
	let runner_id = Uuid::parse_str(&runner_id).unwrap();

	// Register runner if not already registered.
	if !state.runners.lock().unwrap().contains_key(&runner_id) {
		state.register_runner(runner_id);
	}

	if let Some(channel) = state
		.runners
		.lock()
		.unwrap()
		.get_mut(&runner_id)
		.filter(|channel| channel.rejected)
	{
		channel.rejected_attempts += 1;
		return (
			StatusCode::UNAUTHORIZED,
			Json(ApiErrorResponseBody {
				success: False,
				error: ErrorType::Unauthorized,
				message: "Token rejected".to_string(),
			}),
		)
			.into_response();
	}

	let rx = state
		.runners
		.lock()
		.unwrap()
		.get(&runner_id)
		.unwrap()
		.server_tx
		.subscribe();
	let client_msgs = state
		.runners
		.lock()
		.unwrap()
		.get(&runner_id)
		.unwrap()
		.client_msgs
		.clone();

	ws.on_upgrade(move |socket| handle_ws(socket, rx, client_msgs))
		.into_response()
}

async fn handle_ws(
	socket: WebSocket,
	mut server_rx: broadcast::Receiver<String>,
	client_msgs: Arc<Mutex<Vec<StreamRunnerDataForWorkspaceClientMsg>>>,
) {
	let (mut sink, mut stream) = socket.split();

	// Forward server messages to the WS client.
	let mut write_task = tokio::spawn(async move {
		while let Ok(msg) = server_rx.recv().await {
			if sink
				.send(Message::Binary(msg.into_bytes().into()))
				.await
				.is_err()
			{
				break;
			}
		}
	});

	// Collect client messages.
	let mut read_task = tokio::spawn(async move {
		while let Some(Ok(msg)) = stream.next().await {
			let data = match msg {
				Message::Text(t) => t.to_string(),
				Message::Binary(b) => String::from_utf8_lossy(&b).to_string(),
				_ => continue,
			};
			if let Ok(parsed) = serde_json::from_str::<StreamRunnerDataForWorkspaceClientMsg>(&data)
			{
				client_msgs.lock().unwrap().push(parsed);
			}
		}
	});

	tokio::select! {
		_ = &mut write_task => {}
		_ = &mut read_task => {}
	}

	// Each task holds half of the socket, so both must go for the connection
	// to close.
	write_task.abort();
	read_task.abort();
}

async fn list_deployments_handler(
	Path(workspace_id): Path<Uuid>,
	RawQuery(query): RawQuery,
	State(state): State<Arc<ManagedServerState>>,
) -> axum::response::Response {
	if let Some(error) = state.list_error(workspace_id, "deployment") {
		return error;
	}

	// Parsed the way the API parses it, so the nested `search` filter survives.
	let query = serde_qs::from_str::<ListResourceQuery<Deployment, ()>>(
		query.as_deref().unwrap_or_default(),
	)
	.unwrap();
	let deployments = state.deployments.lock().unwrap();
	let all = deployments
		.values()
		.map(|(d, _)| d.clone())
		.filter(|d| {
			query
				.search
				.runner
				.is_none_or(|runner| runner.resource_id == d.runner)
		})
		.collect::<Vec<_>>();
	let total_count = all.len();

	let start = query.page * query.count;
	let page_items = all.into_iter().skip(start).take(query.count).collect();

	(
		TypedHeader(TotalCountHeader(total_count)),
		Json(ApiSuccessResponseBody::new(ListDeploymentResponse {
			deployments: page_items,
		})),
	)
		.into_response()
}

async fn list_managed_urls_handler(
	Path(workspace_id): Path<Uuid>,
	Query(query): Query<ListResourceQuery<ManagedUrl, ()>>,
	State(state): State<Arc<ManagedServerState>>,
) -> axum::response::Response {
	if let Some(error) = state.list_error(workspace_id, "managedUrl") {
		return error;
	}

	let managed_urls = state.managed_urls.lock().unwrap();
	let total_count = managed_urls.len();
	let page_items = managed_urls
		.values()
		.skip(query.page * query.count)
		.take(query.count)
		.cloned()
		.collect();

	(
		TypedHeader(TotalCountHeader(total_count)),
		Json(ApiSuccessResponseBody::new(ListManagedURLResponse {
			urls: page_items,
		})),
	)
		.into_response()
}

async fn get_domain_info_handler(
	Path((_workspace_id, domain_id)): Path<(Uuid, Uuid)>,
	State(state): State<Arc<ManagedServerState>>,
) -> impl IntoResponse {
	let name = state
		.domains
		.lock()
		.unwrap()
		.get(&domain_id)
		.cloned()
		.unwrap();

	Json(ApiSuccessResponseBody::new(
		GetDomainInfoInWorkspaceResponse {
			workspace_domain: WithId::new(
				domain_id,
				WorkspaceDomain {
					name,
					last_verified: None,
					is_verified: true,
				},
			),
		},
	))
}

async fn list_secrets_handler(
	Path(workspace_id): Path<Uuid>,
	Query(query): Query<ListResourceQuery<Secret, ()>>,
	State(state): State<Arc<ManagedServerState>>,
) -> axum::response::Response {
	if let Some(error) = state.list_error(workspace_id, "secret") {
		return error;
	}

	let secrets = state.secrets.lock().unwrap();
	let total_count = secrets.len();
	let page_items = secrets
		.values()
		.skip(query.page * query.count)
		.take(query.count)
		.cloned()
		.collect();

	(
		TypedHeader(TotalCountHeader(total_count)),
		Json(ApiSuccessResponseBody::new(
			ListSecretsForWorkspaceResponse {
				secrets: page_items,
			},
		)),
	)
		.into_response()
}

async fn get_deployment_info_handler(
	Path((_workspace_id, deployment_id)): Path<(Uuid, Uuid)>,
	State(state): State<Arc<ManagedServerState>>,
) -> impl IntoResponse {
	let deployments = state.deployments.lock().unwrap();

	match deployments.get(&deployment_id) {
		Some((deployment, running_details)) => {
			Json(ApiSuccessResponseBody::new(GetDeploymentInfoResponse {
				deployment: deployment.clone(),
				running_details: running_details.clone(),
			}))
			.into_response()
		}
		None => (
			StatusCode::NOT_FOUND,
			Json(ApiErrorResponseBody {
				success: False,
				error: ErrorType::ResourceDoesNotExist,
				message: "Deployment not found".to_string(),
			}),
		)
			.into_response(),
	}
}
