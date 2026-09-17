//! In-test server that impersonates the upstream Patr API for managed mode
//! tests. Handles both WebSocket streams (runner data) and REST endpoints
//! (ListDeployment, GetDeploymentInfo) on port 3000.
//!
//! Each runner connects with a unique `runner_id` in the WS path, so
//! multiple tests can share the same server without interference.

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
		State,
		ws::{Message, WebSocket, WebSocketUpgrade},
	},
	response::IntoResponse,
	routing::get,
};
use axum_extra::TypedHeader;
use common::prelude::*;
use futures::{SinkExt, StreamExt};
use http::StatusCode;
use models::{
	ApiErrorResponseBody,
	api::workspace::{deployment::*, managed_url::*, runner::*},
	utils::False,
};
use tokio::sync::{OnceCell, broadcast};

/// Per-runner state for the test server. Each runner_id gets its own channel
/// pair for sending server messages and receiving client messages.
pub struct RunnerChannel {
	/// Send server messages (DeploymentCreated, etc.) to this runner.
	pub server_tx: broadcast::Sender<String>,
	/// Client messages (Handshake, DeploymentStatusUpdated)
	/// received from this runner.
	pub client_msgs: Arc<Mutex<Vec<StreamRunnerDataForWorkspaceClientMsg>>>,
}

/// Shared state for the test managed server.
pub struct ManagedServerState {
	/// Per-runner channels, keyed by runner_id.
	pub runners: Mutex<BTreeMap<Uuid, RunnerChannel>>,
	/// Deployments to return from ListDeployment/GetDeploymentInfo REST calls,
	/// keyed by deployment_id.
	pub deployments: Mutex<BTreeMap<Uuid, (WithId<Deployment>, DeploymentRunningDetails)>>,
}

impl ManagedServerState {
	pub fn new() -> Arc<Self> {
		Arc::new(Self {
			runners: Mutex::new(BTreeMap::new()),
			deployments: Mutex::new(BTreeMap::new()),
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
			},
		);
		tx
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
					"/workspace/{workspace_id}/infrastructure/managed-url",
					get(list_managed_urls_handler),
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
) -> impl IntoResponse {
	let runner_id = Uuid::parse_str(&runner_id).unwrap();

	// Register runner if not already registered.
	if !state.runners.lock().unwrap().contains_key(&runner_id) {
		state.register_runner(runner_id);
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
}

async fn handle_ws(
	socket: WebSocket,
	mut server_rx: broadcast::Receiver<String>,
	client_msgs: Arc<Mutex<Vec<StreamRunnerDataForWorkspaceClientMsg>>>,
) {
	let (mut sink, mut stream) = socket.split();

	// Forward server messages to the WS client.
	let write_task = tokio::spawn(async move {
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
	let read_task = tokio::spawn(async move {
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
		_ = write_task => {}
		_ = read_task => {}
	}
}

async fn list_deployments_handler(
	Path(_workspace_id): Path<Uuid>,
	Query(query): Query<ListResourceQuery<Deployment, ()>>,
	State(state): State<Arc<ManagedServerState>>,
) -> impl IntoResponse {
	let deployments = state.deployments.lock().unwrap();
	let all = deployments
		.values()
		.map(|(d, _)| d.clone())
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
}

async fn list_managed_urls_handler(
	Path(_workspace_id): Path<Uuid>,
	Query(_query): Query<ListResourceQuery<ManagedUrl, ()>>,
	State(_state): State<Arc<ManagedServerState>>,
) -> impl IntoResponse {
	(
		TypedHeader(TotalCountHeader(0)),
		Json(ApiSuccessResponseBody::new(ListManagedURLResponse {
			urls: Vec::new(),
		})),
	)
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
