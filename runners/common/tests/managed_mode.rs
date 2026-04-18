use std::{collections::BTreeMap, net::SocketAddr, sync::Arc, time::Duration};

use common::{
	actors::runner_supervisor::{RunnerSupervisor, RunnerSupervisorArgs},
	db,
	prelude::*,
};
use models::api::workspace::{deployment::*, runner::*};
use ractor::Actor;

use crate::{
	managed_server::{self, ManagedServerState},
	mock_executor::*,
	utils::periodic_check,
};

/// Set up a managed-mode runner connected to the test server on port 3000.
async fn setup_managed() -> (
	Arc<ManagedServerState>,
	Arc<MockExecutorState>,
	sqlx::Pool<DatabaseType>,
	Uuid,
	Uuid,
	tempfile::TempDir,
) {
	let server_state = managed_server::get_managed_server().await;

	let workspace_id = Uuid::new_v4();
	let runner_id = Uuid::new_v4();

	server_state.register_runner(runner_id);

	let temp_dir = tempfile::TempDir::new().unwrap();
	let db_path = temp_dir.path().join("test.db");

	let mock_state = MockExecutorState::new();

	let config = RunnerSettings::<()> {
		mode: RunnerMode::Managed {
			workspace_id,
			runner_id,
			api_token: "test-token".parse().unwrap(),
			user_agent: "integration-tests/0.1.0".parse().unwrap(),
		},
		environment: RunningEnvironment::Development,
		database: DatabaseConfig {
			file: db_path.to_string_lossy().to_string(),
			connection_limit: 5,
		},
		bind_address: SocketAddr::from(([127, 0, 0, 1], 0)),
		data: (),
	};

	let database = db::connect(&config.database).await.unwrap();
	db::initialize(&database).await.unwrap();

	let (_root_ref, _root_handle) = RunnerSupervisor::<MockExecutor>::spawn(
		Some("runner-supervisor".to_string()),
		RunnerSupervisor::new(),
		RunnerSupervisorArgs {
			config,
			database: database.clone(),
			runner_state: mock_state.clone(),
		},
	)
	.await
	.unwrap();

	// Give the actor tree time to connect.
	tokio::time::sleep(Duration::from_millis(500)).await;

	(
		server_state,
		mock_state,
		database,
		workspace_id,
		runner_id,
		temp_dir,
	)
}

fn test_deployment(runner_id: Uuid) -> (Uuid, Deployment, DeploymentRunningDetails) {
	let id = Uuid::new_v4();
	let deployment = Deployment {
		name: format!("ws-test-dep-{}", &id.to_string()[..8]),
		registry: DeploymentRegistry::ExternalRegistry {
			registry: "docker.io".to_string(),
			image_name: "nginx".to_string(),
		},
		image_tag: "latest".to_string(),
		status: DeploymentStatus::Deploying,
		runner: runner_id,
		current_live_digest: None,
		machine_type: Uuid::parse_str("b3cf3771fa394281bfdfeb2e65a061b6").unwrap(),
	};
	let details = DeploymentRunningDetails {
		deploy_on_push: false,
		min_horizontal_scale: 1,
		max_horizontal_scale: 1,
		ports: BTreeMap::new(),
		environment_variables: BTreeMap::new(),
		startup_probe: None,
		liveness_probe: None,
		config_mounts: BTreeMap::new(),
		volumes: BTreeMap::new(),
	};
	(id, deployment, details)
}

#[tokio::test]
async fn ws_connects_and_sends_exposure_type() {
	let (server_state, _mock, _db, _ws_id, runner_id, _tmp) = setup_managed().await;

	let state = server_state.clone();
	periodic_check(
		move || {
			let msgs = state.get_client_msgs(runner_id);
			msgs.iter().any(|m| {
				matches!(
					m,
					StreamRunnerDataForWorkspaceClientMsg::SetRunnerExposureType { .. }
				)
			})
		},
		Duration::from_secs(5),
	)
	.await;
}

#[tokio::test]
async fn deployment_created_via_ws_triggers_upsert() {
	let (server_state, mock_state, _db, _ws_id, runner_id, _tmp) = setup_managed().await;
	let (dep_id, deployment, details) = test_deployment(runner_id);

	server_state.send_to_runner(
		runner_id,
		&StreamRunnerDataForWorkspaceServerMsg::DeploymentCreated {
			deployment: WithId::new(dep_id, deployment),
			running_details: details,
		},
	);

	let mock = mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Upsert(id) if *id == dep_id)),
		Duration::from_secs(10),
	)
	.await;
}

#[tokio::test]
async fn deployment_deleted_via_ws_triggers_delete() {
	let (server_state, mock_state, _db, _ws_id, runner_id, _tmp) = setup_managed().await;
	let (dep_id, deployment, details) = test_deployment(runner_id);

	server_state.send_to_runner(
		runner_id,
		&StreamRunnerDataForWorkspaceServerMsg::DeploymentCreated {
			deployment: WithId::new(dep_id, deployment),
			running_details: details,
		},
	);

	let mock = mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Upsert(id) if *id == dep_id)),
		Duration::from_secs(10),
	)
	.await;

	mock_state
		.statuses
		.lock()
		.unwrap()
		.insert(dep_id, DeploymentStatus::Running);

	server_state.send_to_runner(
		runner_id,
		&StreamRunnerDataForWorkspaceServerMsg::DeploymentDeleted { id: dep_id },
	);

	let mock = mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Delete(id) if *id == dep_id)),
		Duration::from_secs(10),
	)
	.await;
}

#[tokio::test]
async fn deployment_updated_via_ws_triggers_upsert() {
	let (server_state, mock_state, _db, _ws_id, runner_id, _tmp) = setup_managed().await;
	let (dep_id, deployment, details) = test_deployment(runner_id);

	// Create first.
	server_state.send_to_runner(
		runner_id,
		&StreamRunnerDataForWorkspaceServerMsg::DeploymentCreated {
			deployment: WithId::new(dep_id, deployment.clone()),
			running_details: details.clone(),
		},
	);

	let mock = mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Upsert(id) if *id == dep_id)),
		Duration::from_secs(10),
	)
	.await;

	// Now send update with a different image tag.
	mock_state.calls.lock().unwrap().clear();
	let mut updated = deployment;
	updated.image_tag = "v2".to_string();

	server_state.send_to_runner(
		runner_id,
		&StreamRunnerDataForWorkspaceServerMsg::DeploymentUpdated {
			deployment: WithId::new(dep_id, updated),
			running_details: details,
		},
	);

	let mock = mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Upsert(id) if *id == dep_id)),
		Duration::from_secs(10),
	)
	.await;
}

#[tokio::test]
async fn status_change_flows_upstream_via_ws() {
	let (server_state, mock_state, _db, _ws_id, runner_id, _tmp) = setup_managed().await;
	let (dep_id, deployment, details) = test_deployment(runner_id);

	// Create a deployment — mock reports Running so the actor will report
	// status change after the first CheckStatus.
	mock_state
		.statuses
		.lock()
		.unwrap()
		.insert(dep_id, DeploymentStatus::Running);

	server_state.send_to_runner(
		runner_id,
		&StreamRunnerDataForWorkspaceServerMsg::DeploymentCreated {
			deployment: WithId::new(dep_id, deployment),
			running_details: details,
		},
	);

	// Wait for the status to be reported upstream via WS.
	let state = server_state.clone();
	periodic_check(
		move || {
			let msgs = state.get_client_msgs(runner_id);
			msgs.iter().any(|m| {
				matches!(
					m,
					StreamRunnerDataForWorkspaceClientMsg::DeploymentStatusUpdated {
						id,
						status: DeploymentStatus::Running,
					} if *id == dep_id
				)
			})
		},
		Duration::from_secs(15),
	)
	.await;
}

#[tokio::test]
async fn full_resync_adds_missing_deployment() {
	let (server_state, mock_state, database, _ws_id, runner_id, _tmp) = setup_managed().await;
	let (dep_id, deployment, details) = test_deployment(runner_id);

	// Add a deployment to the mock REST server that the runner doesn't
	// know about. On the next full resync (30s in debug mode), the
	// runner should fetch it via ListDeployment + GetDeploymentInfo and
	// write it to SQLite.
	server_state.add_deployment(WithId::new(dep_id, deployment), details);

	// Wait for the full resync to pick it up and the actor to upsert.
	let mock = mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Upsert(id) if *id == dep_id)),
		Duration::from_secs(45),
	)
	.await;

	// Verify it's in SQLite.
	let row = sqlx::query("SELECT id FROM deployment WHERE id = $1")
		.bind(dep_id)
		.fetch_optional(&database)
		.await
		.unwrap();
	assert!(
		row.is_some(),
		"deployment should be in SQLite after full resync"
	);
}
