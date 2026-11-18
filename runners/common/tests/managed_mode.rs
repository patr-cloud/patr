use std::{
	collections::{BTreeMap, BTreeSet},
	sync::Arc,
	time::{Duration, Instant},
};

use common::{
	actors::{
		db_helpers,
		runner_supervisor::{RunnerSupervisor, RunnerSupervisorArgs},
	},
	db,
	prelude::*,
};
use http::StatusCode;
use models::api::workspace::{
	deployment::*,
	managed_url::{ManagedUrl, ManagedUrlType},
	runner::*,
	secret::Secret,
};
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
		workspace_id,
		runner_id,
		api_token: "test-token".parse().unwrap(),
		user_agent: "integration-tests/0.1.0".parse().unwrap(),
		environment: RunningEnvironment::Development,
		database: DatabaseConfig {
			file: db_path.to_string_lossy().to_string(),
			connection_limit: 5,
		},
		data: (),
	};

	let database = db::connect(&config.database).await.unwrap();
	db::initialize(&database, &config).await.unwrap();

	let (_root_ref, _root_handle) = RunnerSupervisor::<MockExecutor>::spawn(
		Some("runner-supervisor".to_string()),
		RunnerSupervisor::new(),
		RunnerSupervisorArgs {
			config,
			database: database.clone(),
			executor: MockExecutor {
				state: mock_state.clone(),
			},
			started: false,
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
		status: DeploymentStatus::Observed(ObservedDeploymentStatus::Deploying),
		should_run: true,
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
async fn ws_connects_and_sends_handshake() {
	let (server_state, _mock, _db, _ws_id, runner_id, _tmp) = setup_managed().await;

	let state = server_state.clone();
	periodic_check(
		move || {
			let msgs = state.get_client_msgs(runner_id);
			msgs.iter()
				.any(|m| matches!(m, StreamRunnerDataForWorkspaceClientMsg::Handshake { .. }))
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
		&StreamRunnerDataForWorkspaceServerMsg::DeploymentUpdated {
			deployment: WithId::new(dep_id, deployment),
			running_details: details,
		},
	);

	let mock = mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Apply(id) if *id == service(dep_id))),
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
		&StreamRunnerDataForWorkspaceServerMsg::DeploymentUpdated {
			deployment: WithId::new(dep_id, deployment),
			running_details: details,
		},
	);

	let mock = mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Apply(id) if *id == service(dep_id))),
		Duration::from_secs(10),
	)
	.await;

	mock_state
		.statuses
		.lock()
		.unwrap()
		.insert(dep_id, ObservedDeploymentStatus::Running);

	server_state.send_to_runner(
		runner_id,
		&StreamRunnerDataForWorkspaceServerMsg::DeploymentDeleted { id: dep_id },
	);

	let mock = mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Delete(id) if *id == service(dep_id))),
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
		&StreamRunnerDataForWorkspaceServerMsg::DeploymentUpdated {
			deployment: WithId::new(dep_id, deployment.clone()),
			running_details: details.clone(),
		},
	);

	let mock = mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Apply(id) if *id == service(dep_id))),
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
		move || mock.has_call(|c| matches!(c, ExecutorCall::Apply(id) if *id == service(dep_id))),
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
		.insert(dep_id, ObservedDeploymentStatus::Running);

	server_state.send_to_runner(
		runner_id,
		&StreamRunnerDataForWorkspaceServerMsg::DeploymentUpdated {
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
						status: ObservedDeploymentStatus::Running,
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
		move || mock.has_call(|c| matches!(c, ExecutorCall::Apply(id) if *id == service(dep_id))),
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

/// A deployment whose only env var reads `secret_id`.
fn test_deployment_with_secret(
	runner_id: Uuid,
	secret_id: Uuid,
) -> (Uuid, Deployment, DeploymentRunningDetails) {
	let (id, deployment, mut details) = test_deployment(runner_id);
	details.environment_variables.insert(
		"API_KEY".to_string(),
		EnvironmentVariableValue::Secret {
			from_secret: secret_id,
		},
	);
	(id, deployment, details)
}

#[tokio::test]
async fn secret_updated_via_ws_reapplies_deployment() {
	let (server_state, mock_state, _db, _ws_id, runner_id, _tmp) = setup_managed().await;
	let secret_id = Uuid::new_v4();
	let (dep_id, deployment, details) = test_deployment_with_secret(runner_id, secret_id);
	let (other_id, other, other_details) = test_deployment(runner_id);

	for (id, deployment, details) in [
		(dep_id, deployment, details),
		(other_id, other, other_details),
	] {
		server_state.send_to_runner(
			runner_id,
			&StreamRunnerDataForWorkspaceServerMsg::DeploymentUpdated {
				deployment: WithId::new(id, deployment),
				running_details: details,
			},
		);
	}

	let mock = mock_state.clone();
	periodic_check(
		move || {
			mock.has_call(|c| matches!(c, ExecutorCall::Apply(id) if *id == service(dep_id))) &&
				mock.has_call(
					|c| matches!(c, ExecutorCall::Apply(id) if *id == service(other_id)),
				)
		},
		Duration::from_secs(10),
	)
	.await;

	// Report both as running so the status poll doesn't re-apply them.
	mock_state.statuses.lock().unwrap().extend([
		(dep_id, ObservedDeploymentStatus::Running),
		(other_id, ObservedDeploymentStatus::Running),
	]);
	mock_state.calls.lock().unwrap().clear();
	server_state.send_to_runner(
		runner_id,
		&StreamRunnerDataForWorkspaceServerMsg::SecretUpdated {
			id: secret_id,
			last_updated: time::OffsetDateTime::now_utc(),
		},
	);

	let mock = mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Apply(id) if *id == service(dep_id))),
		Duration::from_secs(10),
	)
	.await;

	// Only the deployment that reads the secret is re-applied.
	assert!(
		!mock_state.has_call(|c| matches!(c, ExecutorCall::Apply(id) if *id == service(other_id))),
		"a deployment that doesn't use the secret must not be re-applied"
	);
}

#[tokio::test]
async fn full_resync_reapplies_deployment_after_missed_rotation() {
	let (server_state, mock_state, database, _ws_id, runner_id, _tmp) = setup_managed().await;
	let secret_id = Uuid::new_v4();
	let (dep_id, deployment, details) = test_deployment_with_secret(runner_id, secret_id);

	server_state.send_to_runner(
		runner_id,
		&StreamRunnerDataForWorkspaceServerMsg::DeploymentUpdated {
			deployment: WithId::new(dep_id, deployment.clone()),
			running_details: details.clone(),
		},
	);

	let mock = mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Apply(id) if *id == service(dep_id))),
		Duration::from_secs(10),
	)
	.await;

	// Report it as running so the status poll doesn't re-apply it.
	mock_state
		.statuses
		.lock()
		.unwrap()
		.insert(dep_id, ObservedDeploymentStatus::Running);

	// The secret rotates upstream without a `SecretUpdated` reaching the
	// runner. The next full resync (30s in debug mode) should notice the new
	// version and re-apply the deployment.
	mock_state.calls.lock().unwrap().clear();
	let last_updated = time::OffsetDateTime::now_utc();
	server_state.add_deployment(WithId::new(dep_id, deployment), details);
	server_state.add_secret(WithId::new(
		secret_id,
		Secret {
			name: "API_KEY".to_string(),
			created: last_updated,
			last_updated,
		},
	));

	let mock = mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Apply(id) if *id == service(dep_id))),
		Duration::from_secs(45),
	)
	.await;

	let row = sqlx::query("SELECT last_updated FROM secret WHERE id = $1")
		.bind(secret_id)
		.fetch_one(&database)
		.await
		.unwrap();
	assert_eq!(
		row.try_get::<time::OffsetDateTime, _>("last_updated")
			.unwrap(),
		last_updated,
		"full resync should record the secret's upstream version"
	);
}

#[tokio::test]
async fn stopped_deployment_is_not_started_by_rotation() {
	let (server_state, mock_state, _db, _ws_id, runner_id, _tmp) = setup_managed().await;
	let secret_id = Uuid::new_v4();
	let (dep_id, mut deployment, details) = test_deployment_with_secret(runner_id, secret_id);
	deployment.status = DeploymentStatus::Observed(ObservedDeploymentStatus::Stopped);
	deployment.should_run = false;

	server_state.send_to_runner(
		runner_id,
		&StreamRunnerDataForWorkspaceServerMsg::DeploymentUpdated {
			deployment: WithId::new(dep_id, deployment),
			running_details: details,
		},
	);
	server_state.send_to_runner(
		runner_id,
		&StreamRunnerDataForWorkspaceServerMsg::SecretUpdated {
			id: secret_id,
			last_updated: time::OffsetDateTime::now_utc(),
		},
	);

	// Long enough for the rotation to be handled and a status poll to run.
	tokio::time::sleep(Duration::from_secs(7)).await;

	assert!(
		!mock_state.has_call(|c| matches!(c, ExecutorCall::Apply(id) if *id == service(dep_id))),
		"a stopped deployment must not be started, not even briefly"
	);
}

#[tokio::test]
async fn rejected_token_keeps_retrying_without_touching_deployments() {
	let (server_state, mock_state, database, _ws_id, runner_id, _tmp) = setup_managed().await;
	let (dep_id, deployment, details) = test_deployment(runner_id);

	// Upstream keeps the deployment, so the resync after recovering keeps it
	// too.
	server_state.add_deployment(WithId::new(dep_id, deployment.clone()), details.clone());
	server_state.send_to_runner(
		runner_id,
		&StreamRunnerDataForWorkspaceServerMsg::DeploymentUpdated {
			deployment: WithId::new(dep_id, deployment),
			running_details: details,
		},
	);

	let mock = mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Apply(id) if *id == service(dep_id))),
		Duration::from_secs(10),
	)
	.await;

	mock_state.calls.lock().unwrap().clear();
	let handshakes = || {
		server_state
			.get_client_msgs(runner_id)
			.iter()
			.filter(|m| matches!(m, StreamRunnerDataForWorkspaceClientMsg::Handshake { .. }))
			.count()
	};
	let handshakes_before = handshakes();

	// The token is regenerated: upstream drops the stream and turns every
	// reconnect away.
	server_state.set_rejected(runner_id, true);

	let state = server_state.clone();
	periodic_check(
		move || state.rejected_attempts(runner_id) >= 2,
		Duration::from_secs(10),
	)
	.await;

	let assert_untouched = async || {
		let row = sqlx::query("SELECT id FROM deployment WHERE id = $1")
			.bind(dep_id)
			.fetch_optional(&database)
			.await
			.unwrap();
		assert!(row.is_some(), "the deployment should still be in SQLite");
		assert!(
			!mock_state.has_call(|c| matches!(
				c,
				ExecutorCall::Apply(id) | ExecutorCall::Delete(id)
					if id.contains(&dep_id.to_string())
			)),
			"the deployment should not have been touched"
		);
	};
	assert_untouched().await;

	// A new token is in place: the runner reconnects on its own.
	server_state.set_rejected(runner_id, false);

	periodic_check(|| handshakes() > handshakes_before, Duration::from_secs(20)).await;

	// Give the resync that follows the reconnect time to land.
	tokio::time::sleep(Duration::from_secs(2)).await;
	assert_untouched().await;
}

/// Polls `check` every 50ms until it returns true, or panics after `timeout`.
async fn wait_until(check: impl AsyncFn() -> bool, timeout: Duration) {
	let start = Instant::now();
	while start.elapsed() < timeout {
		if check().await {
			return;
		}
		tokio::time::sleep(Duration::from_millis(50)).await;
	}
	assert!(check().await, "condition not met after {timeout:?}");
}

/// Whether `sql`, bound to `id`, returns a row.
async fn row_exists(database: &sqlx::Pool<DatabaseType>, sql: &str, id: Uuid) -> bool {
	sqlx::query(sqlx::AssertSqlSafe(sql.to_owned()))
		.bind(id)
		.fetch_optional(database)
		.await
		.unwrap()
		.is_some()
}

/// How many times the runner reported `status` for a deployment.
fn status_reports(
	server_state: &ManagedServerState,
	runner_id: Uuid,
	deployment_id: Uuid,
	status: ObservedDeploymentStatus,
) -> usize {
	server_state
		.get_client_msgs(runner_id)
		.iter()
		.filter(|msg| {
			matches!(
				msg,
				StreamRunnerDataForWorkspaceClientMsg::DeploymentStatusUpdated {
					id,
					status: reported,
				} if *id == deployment_id && *reported == status
			)
		})
		.count()
}

#[tokio::test]
async fn deployment_updated_for_another_runner_is_ignored() {
	let (server_state, mock_state, database, _ws_id, runner_id, _tmp) = setup_managed().await;
	let (other_id, other, other_details) = test_deployment(Uuid::new_v4());
	let (dep_id, deployment, details) = test_deployment(runner_id);

	for (id, deployment, details) in [
		(other_id, other, other_details),
		(dep_id, deployment, details),
	] {
		server_state.send_to_runner(
			runner_id,
			&StreamRunnerDataForWorkspaceServerMsg::DeploymentUpdated {
				deployment: WithId::new(id, deployment),
				running_details: details,
			},
		);
	}

	// Events are handled in order, so once this runner's deployment is
	// applied, the other runner's has been handled too.
	let mock = mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Apply(id) if *id == service(dep_id))),
		Duration::from_secs(10),
	)
	.await;

	assert!(
		!row_exists(
			&database,
			"SELECT 1 FROM deployment WHERE id = $1",
			other_id
		)
		.await
	);
	assert!(
		!mock_state.has_call(|c| matches!(c, ExecutorCall::Apply(id) if *id == service(other_id)))
	);
}

#[tokio::test]
async fn deployment_deleted_via_ws_removes_rows_and_managed_urls() {
	let (server_state, mock_state, database, _ws_id, runner_id, _tmp) = setup_managed().await;
	let (dep_id, deployment, mut details) = test_deployment(runner_id);
	details
		.ports
		.insert(StringifiedU16::new(80), ExposedPortType::Http);
	details.startup_probe = Some(DeploymentProbe {
		port: 80,
		path: "/health".to_string(),
	});
	details.environment_variables.insert(
		"KEY".to_string(),
		EnvironmentVariableValue::String("value".to_string()),
	);
	details.config_mounts.insert(
		"/etc/app.conf".to_string(),
		Base64String::from(b"key".to_vec()),
	);
	details.volumes.insert("/data".to_string(), VolumeConfig {});

	mock_state
		.statuses
		.lock()
		.unwrap()
		.insert(dep_id, ObservedDeploymentStatus::Running);
	server_state.send_to_runner(
		runner_id,
		&StreamRunnerDataForWorkspaceServerMsg::DeploymentUpdated {
			deployment: WithId::new(dep_id, deployment),
			running_details: details,
		},
	);

	let mock = mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Apply(id) if *id == service(dep_id))),
		Duration::from_secs(10),
	)
	.await;

	let managed_url_id = Uuid::new_v4();
	let mut conn = database.acquire().await.unwrap();
	db_helpers::upsert_managed_url_in_database(
		&mut conn,
		managed_url_id,
		"app.example.com",
		"/",
		dep_id,
		80,
	)
	.await
	.unwrap();
	drop(conn);

	server_state.send_to_runner(
		runner_id,
		&StreamRunnerDataForWorkspaceServerMsg::DeploymentDeleted { id: dep_id },
	);

	// The deployment is torn down, and its managed URL's row goes with the
	// deployment's rows.
	let mock = mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Delete(id) if *id == service(dep_id))),
		Duration::from_secs(10),
	)
	.await;

	for table in [
		"deployment_exposed_port",
		"deployment_environment_variable",
		"deployment_config_mounts",
		"deployment_volume",
	] {
		assert!(
			!row_exists(
				&database,
				&format!("SELECT 1 FROM {table} WHERE deployment_id = $1"),
				dep_id
			)
			.await,
			"rows left in {table}"
		);
	}
	assert!(!row_exists(&database, "SELECT 1 FROM deployment WHERE id = $1", dep_id).await);
	assert!(
		!row_exists(
			&database,
			"SELECT 1 FROM managed_url WHERE id = $1",
			managed_url_id
		)
		.await
	);
}

/// #314: a status is sent again on the next connection, whose resync sends
/// every status, but isn't repeated on the same one while nothing changes.
#[tokio::test]
async fn status_is_resent_after_reconnect() {
	let (server_state, mock_state, _db, _ws_id, runner_id, _tmp) = setup_managed().await;
	let (dep_id, deployment, details) = test_deployment(runner_id);

	mock_state
		.statuses
		.lock()
		.unwrap()
		.insert(dep_id, ObservedDeploymentStatus::Running);
	// Listed by the API, so the resync on reconnect keeps it.
	server_state.add_deployment(WithId::new(dep_id, deployment.clone()), details.clone());
	server_state.send_to_runner(
		runner_id,
		&StreamRunnerDataForWorkspaceServerMsg::DeploymentUpdated {
			deployment: WithId::new(dep_id, deployment),
			running_details: details,
		},
	);

	let state = server_state.clone();
	periodic_check(
		move || status_reports(&state, runner_id, dep_id, ObservedDeploymentStatus::Running) >= 1,
		Duration::from_secs(15),
	)
	.await;

	// Another status check passes without a change, so nothing is resent.
	tokio::time::sleep(Duration::from_secs(6)).await;
	assert_eq!(
		status_reports(
			&server_state,
			runner_id,
			dep_id,
			ObservedDeploymentStatus::Running
		),
		1
	);

	server_state.disconnect_runner(runner_id);

	let state = server_state.clone();
	periodic_check(
		move || status_reports(&state, runner_id, dep_id, ObservedDeploymentStatus::Running) >= 2,
		Duration::from_secs(15),
	)
	.await;
}

/// #314: a stop then a start on one connection reports both, even though the
/// deployment ends up where it started.
#[tokio::test]
async fn stop_then_start_on_same_connection_sends_both() {
	let (server_state, mock_state, _db, _ws_id, runner_id, _tmp) = setup_managed().await;
	let (dep_id, deployment, details) = test_deployment(runner_id);

	mock_state
		.statuses
		.lock()
		.unwrap()
		.insert(dep_id, ObservedDeploymentStatus::Running);
	server_state.send_to_runner(
		runner_id,
		&StreamRunnerDataForWorkspaceServerMsg::DeploymentUpdated {
			deployment: WithId::new(dep_id, deployment.clone()),
			running_details: details.clone(),
		},
	);

	let state = server_state.clone();
	periodic_check(
		move || status_reports(&state, runner_id, dep_id, ObservedDeploymentStatus::Running) >= 1,
		Duration::from_secs(15),
	)
	.await;

	// Stop: the API now shows Stopping, until the runner reports Stopped.
	let mut stopping = deployment.clone();
	stopping.should_run = false;
	stopping.status = DeploymentStatus::Stopping;
	server_state.send_to_runner(
		runner_id,
		&StreamRunnerDataForWorkspaceServerMsg::DeploymentUpdated {
			deployment: WithId::new(dep_id, stopping),
			running_details: details.clone(),
		},
	);

	let mock = mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Delete(id) if *id == service(dep_id))),
		Duration::from_secs(10),
	)
	.await;
	mock_state
		.statuses
		.lock()
		.unwrap()
		.insert(dep_id, ObservedDeploymentStatus::Stopped);

	let state = server_state.clone();
	periodic_check(
		move || status_reports(&state, runner_id, dep_id, ObservedDeploymentStatus::Stopped) >= 1,
		Duration::from_secs(15),
	)
	.await;

	// Start: the API shows Deploying until the runner reports Running again.
	let mut deploying = deployment;
	deploying.should_run = true;
	deploying.status = DeploymentStatus::Observed(ObservedDeploymentStatus::Deploying);
	server_state.send_to_runner(
		runner_id,
		&StreamRunnerDataForWorkspaceServerMsg::DeploymentUpdated {
			deployment: WithId::new(dep_id, deploying),
			running_details: details,
		},
	);

	let mock = mock_state.clone();
	periodic_check(
		move || {
			mock.call_count(|c| matches!(c, ExecutorCall::Apply(id) if *id == service(dep_id))) >= 2
		},
		Duration::from_secs(10),
	)
	.await;
	mock_state
		.statuses
		.lock()
		.unwrap()
		.insert(dep_id, ObservedDeploymentStatus::Running);

	let state = server_state.clone();
	periodic_check(
		move || status_reports(&state, runner_id, dep_id, ObservedDeploymentStatus::Running) >= 2,
		Duration::from_secs(15),
	)
	.await;
}

#[tokio::test]
async fn full_resync_applies_only_the_differences() {
	let (server_state, _mock_state, database, _ws_id, runner_id, _tmp) = setup_managed().await;

	// Every deployment carries an env var, so rewriting one re-inserts it.
	let mut deployments = Vec::new();
	for _ in 0..3 {
		let (id, deployment, mut details) = test_deployment(runner_id);
		details.environment_variables.insert(
			"KEY".to_string(),
			EnvironmentVariableValue::String("value".to_string()),
		);
		server_state.add_deployment(WithId::new(id, deployment.clone()), details.clone());
		deployments.push((id, deployment, details));
	}
	let [
		(unchanged_id, ..),
		(changed_id, mut changed, changed_details),
		(removed_id, ..),
	] = <[_; 3]>::try_from(deployments).unwrap();
	// Another runner's deployment, which the server-side filter leaves out.
	let (other_id, other, other_details) = test_deployment(Uuid::new_v4());
	server_state.add_deployment(WithId::new(other_id, other), other_details);

	let [kept_url_id, removed_url_id] = [Uuid::new_v4(), Uuid::new_v4()];
	for (managed_url_id, deployment_id) in
		[(kept_url_id, unchanged_id), (removed_url_id, removed_id)]
	{
		server_state.add_managed_url(
			WithId::new(
				managed_url_id,
				ManagedUrl {
					sub_domain: "app".to_string(),
					domain_id: Uuid::new_v4(),
					path: "/".to_string(),
					url_type: ManagedUrlType::ProxyDeployment {
						deployment_id,
						port: 80,
					},
					is_active: true,
				},
			),
			"example.com",
		);
	}

	let [rotated_secret_id, kept_secret_id, removed_secret_id] =
		[Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
	let created = time::OffsetDateTime::now_utc();
	for secret_id in [rotated_secret_id, kept_secret_id, removed_secret_id] {
		server_state.add_secret(WithId::new(
			secret_id,
			Secret {
				name: format!("SECRET_{secret_id}"),
				created,
				last_updated: created,
			},
		));
	}

	server_state.disconnect_runner(runner_id);
	wait_until(
		async || {
			row_exists(
				&database,
				"SELECT 1 FROM deployment WHERE id = $1",
				removed_id,
			)
			.await &&
				row_exists(
					&database,
					"SELECT 1 FROM managed_url WHERE id = $1",
					removed_url_id,
				)
				.await &&
				row_exists(
					&database,
					"SELECT 1 FROM secret WHERE id = $1",
					removed_secret_id,
				)
				.await
		},
		Duration::from_secs(15),
	)
	.await;
	assert!(
		!row_exists(
			&database,
			"SELECT 1 FROM deployment WHERE id = $1",
			other_id
		)
		.await
	);
	assert_eq!(
		sqlx::query("SELECT host FROM managed_url WHERE id = $1")
			.bind(kept_url_id)
			.fetch_one(&database)
			.await
			.unwrap()
			.get::<String, _>("host"),
		"app.example.com"
	);

	// From here on, log every row the resync writes.
	for statement in [
		"CREATE TABLE write_log(id TEXT NOT NULL)",
		"CREATE TRIGGER log_env AFTER INSERT ON deployment_environment_variable BEGIN INSERT INTO write_log VALUES (NEW.deployment_id); END",
		"CREATE TRIGGER log_url_insert AFTER INSERT ON managed_url BEGIN INSERT INTO write_log VALUES (NEW.id); END",
		"CREATE TRIGGER log_url_update AFTER UPDATE ON managed_url BEGIN INSERT INTO write_log VALUES (NEW.id); END",
		"CREATE TRIGGER log_secret_insert AFTER INSERT ON secret BEGIN INSERT INTO write_log VALUES (NEW.id); END",
		"CREATE TRIGGER log_secret_update AFTER UPDATE ON secret BEGIN INSERT INTO write_log VALUES (NEW.id); END",
	] {
		sqlx::query(statement).execute(&database).await.unwrap();
	}

	changed.image_tag = "v2".to_string();
	server_state.add_deployment(WithId::new(changed_id, changed), changed_details);
	server_state.remove_deployment(removed_id);
	let rotated = time::OffsetDateTime::now_utc();
	server_state.add_secret(WithId::new(
		rotated_secret_id,
		Secret {
			name: format!("SECRET_{rotated_secret_id}"),
			created,
			last_updated: rotated,
		},
	));
	server_state.remove_secret(removed_secret_id);

	server_state.disconnect_runner(runner_id);
	wait_until(
		async || {
			!row_exists(
				&database,
				"SELECT 1 FROM deployment WHERE id = $1",
				removed_id,
			)
			.await
		},
		Duration::from_secs(15),
	)
	.await;

	// Only the changed deployment and the rotated secret were written.
	let written = sqlx::query("SELECT id FROM write_log")
		.fetch_all(&database)
		.await
		.unwrap()
		.into_iter()
		.map(|row| row.get::<Uuid, _>("id"))
		.collect::<BTreeSet<_>>();
	assert_eq!(written, [changed_id, rotated_secret_id].into());

	assert_eq!(
		sqlx::query("SELECT image_tag FROM deployment WHERE id = $1")
			.bind(changed_id)
			.fetch_one(&database)
			.await
			.unwrap()
			.get::<String, _>("image_tag"),
		"v2"
	);
	assert!(
		row_exists(
			&database,
			"SELECT 1 FROM deployment WHERE id = $1",
			unchanged_id
		)
		.await
	);
	assert!(
		row_exists(
			&database,
			"SELECT 1 FROM managed_url WHERE id = $1",
			kept_url_id
		)
		.await
	);
	assert!(
		!row_exists(
			&database,
			"SELECT 1 FROM managed_url WHERE id = $1",
			removed_url_id
		)
		.await
	);
	assert!(
		row_exists(
			&database,
			"SELECT 1 FROM secret WHERE id = $1",
			kept_secret_id
		)
		.await
	);
	assert!(
		!row_exists(
			&database,
			"SELECT 1 FROM secret WHERE id = $1",
			removed_secret_id
		)
		.await
	);
	assert_eq!(
		sqlx::query("SELECT last_updated FROM secret WHERE id = $1")
			.bind(rotated_secret_id)
			.fetch_one(&database)
			.await
			.unwrap()
			.get::<time::OffsetDateTime, _>("last_updated"),
		rotated
	);
}

/// A list call failing mid-resync leaves Local as it was, even for resources
/// that are gone upstream.
async fn assert_failed_list_keeps_local_state(list: &'static str, status: StatusCode) {
	let (server_state, _mock_state, database, workspace_id, runner_id, _tmp) =
		setup_managed().await;
	let (dep_id, deployment, details) = test_deployment(runner_id);
	let secret_id = Uuid::new_v4();
	let now = time::OffsetDateTime::now_utc();
	server_state.add_deployment(WithId::new(dep_id, deployment), details);
	server_state.add_secret(WithId::new(
		secret_id,
		Secret {
			name: "API_KEY".to_string(),
			created: now,
			last_updated: now,
		},
	));

	server_state.disconnect_runner(runner_id);
	wait_until(
		async || {
			row_exists(&database, "SELECT 1 FROM deployment WHERE id = $1", dep_id).await &&
				row_exists(&database, "SELECT 1 FROM secret WHERE id = $1", secret_id).await
		},
		Duration::from_secs(15),
	)
	.await;

	server_state.remove_deployment(dep_id);
	server_state.remove_secret(secret_id);
	server_state.fail_list(workspace_id, list, status);

	server_state.disconnect_runner(runner_id);
	let state = server_state.clone();
	periodic_check(
		move || state.list_failures_served(workspace_id, list) >= 1,
		Duration::from_secs(15),
	)
	.await;
	tokio::time::sleep(Duration::from_secs(1)).await;

	assert!(row_exists(&database, "SELECT 1 FROM deployment WHERE id = $1", dep_id).await);
	assert!(row_exists(&database, "SELECT 1 FROM secret WHERE id = $1", secret_id).await);

	// Once every list succeeds, the next resync goes through.
	server_state.clear_list_errors(workspace_id);
	server_state.disconnect_runner(runner_id);
	wait_until(
		async || {
			!row_exists(&database, "SELECT 1 FROM deployment WHERE id = $1", dep_id).await &&
				!row_exists(&database, "SELECT 1 FROM secret WHERE id = $1", secret_id).await
		},
		Duration::from_secs(15),
	)
	.await;
}

#[tokio::test]
async fn full_resync_keeps_local_state_when_a_list_is_rate_limited() {
	assert_failed_list_keeps_local_state("deployment", StatusCode::TOO_MANY_REQUESTS).await;
}

#[tokio::test]
async fn full_resync_keeps_local_state_when_a_later_list_fails() {
	assert_failed_list_keeps_local_state("secret", StatusCode::SERVICE_UNAVAILABLE).await;
}

/// Reconcile a deployment with a volume, then delete it from the local state,
/// with the API knowing it on the runner `api_runner` picks given this
/// runner's ID, or not at all. Returns the reconcile's outcome after the
/// delete.
async fn reconcile_deleted_deployment_with_volume(
	api_runner: impl FnOnce(Uuid) -> Option<Uuid>,
) -> (
	crate::setup::TestSetup,
	Uuid,
	common::actors::controller::Outcome,
) {
	let server_state = managed_server::get_managed_server().await;
	let setup = crate::setup::setup().await;
	let (id, deployment, mut details) = test_deployment(setup.config.runner_id);
	details.volumes.insert("/data".to_string(), VolumeConfig {});
	setup
		.upsert_deployment(id, deployment.clone(), details.clone())
		.await;
	setup.reconcile(Owner::Deployment(id)).await;
	assert!(setup.mock_state.exists(&volume(id, "/data")));

	if let Some(runner) = api_runner(setup.config.runner_id) {
		server_state.add_deployment(
			WithId::new(
				id,
				Deployment {
					runner,
					..deployment
				},
			),
			details,
		);
	}
	setup.delete_deployment(id).await;
	let outcome = setup.reconcile(Owner::Deployment(id)).await;
	(setup, id, outcome)
}

/// A deleted deployment's volume is deleted once the API confirms the
/// deployment is gone, and its finalizer's record goes with it.
#[tokio::test]
async fn a_deleted_deployments_volume_is_reaped() {
	let (setup, id, outcome) = reconcile_deleted_deployment_with_volume(|_| None).await;

	assert!(!outcome.waiting);
	assert!(!setup.mock_state.exists(&volume(id, "/data")));
	assert!(
		!row_exists(
			&setup.database,
			"SELECT 1 FROM object_finalizer WHERE deployment_id = $1",
			id
		)
		.await
	);
}

/// A deployment that moved to another runner has its volume deleted here.
#[tokio::test]
async fn a_moved_deployments_volume_is_reaped() {
	let (setup, id, outcome) =
		reconcile_deleted_deployment_with_volume(|_| Some(Uuid::new_v4())).await;

	assert!(!outcome.waiting);
	assert!(!setup.mock_state.exists(&volume(id, "/data")));
}

/// While the API still says the deployment is on this runner, its volume is
/// kept, whatever the local state says, and the finalizer records why.
#[tokio::test]
async fn a_volume_is_kept_while_the_api_says_its_deployment_is_here() {
	let (setup, id, outcome) = reconcile_deleted_deployment_with_volume(Some).await;

	assert!(outcome.waiting);
	assert!(setup.mock_state.exists(&volume(id, "/data")));
	let row =
		sqlx::query("SELECT state, last_error FROM object_finalizer WHERE deployment_id = $1")
			.bind(id)
			.fetch_one(&setup.database)
			.await
			.unwrap();
	assert_eq!(row.get::<String, _>("state"), "failed");
	assert_eq!(
		row.get::<Option<String>, _>("last_error").as_deref(),
		Some("still_on_this_runner")
	);
}
