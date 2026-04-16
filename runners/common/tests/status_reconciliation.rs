use crate::prelude::*;

/// Helper: set up a deployment with a given DB status and mock executor status,
/// then wait for the actor to process at least one CheckStatus cycle.
async fn setup_with_statuses(
	db_status: DeploymentStatus,
	executor_status: DeploymentStatus,
) -> (TestSetup, Uuid) {
	let setup = setup().await;
	let id = setup.create_test_deployment_with_status(db_status).await;

	setup
		.mock_state
		.statuses
		.lock()
		.unwrap()
		.insert(id, executor_status);

	setup.notify_upsert(id);

	// Wait for the initial upsert to complete.
	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Upsert(i) if *i == id)),
		Duration::from_secs(5),
	)
	.await;

	// Wait for at least one status check cycle.
	setup.mock_state.calls.lock().unwrap().clear();
	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::GetStatus(i) if *i == id)),
		Duration::from_secs(10),
	)
	.await;

	(setup, id)
}

#[tokio::test]
async fn running_matches_running_is_noop() {
	let (setup, id) = setup_with_statuses(
		DeploymentStatus::Running,
		DeploymentStatus::Running,
	)
	.await;

	// No upsert or delete should have been called after the status check.
	assert!(
		!setup.mock_state.has_call(|c| matches!(c, ExecutorCall::Upsert(i) if *i == id)),
		"expected no upsert when statuses match"
	);
	assert!(
		!setup.mock_state.has_call(|c| matches!(c, ExecutorCall::Delete(i) if *i == id)),
		"expected no delete when statuses match"
	);
}

#[tokio::test]
async fn deploying_in_db_running_from_executor_updates_db() {
	let (setup, id) = setup_with_statuses(
		DeploymentStatus::Deploying,
		DeploymentStatus::Running,
	)
	.await;

	// DB should be updated to Running.
	let row = sqlx::query("SELECT status FROM deployment WHERE id = $1")
		.bind(id)
		.fetch_one(&setup.database)
		.await
		.unwrap();
	let status: String = row.get("status");
	assert_eq!(status, "running");
}

#[tokio::test]
async fn errored_in_db_running_from_executor_updates_db() {
	let (setup, id) = setup_with_statuses(
		DeploymentStatus::Errored,
		DeploymentStatus::Running,
	)
	.await;

	let row = sqlx::query("SELECT status FROM deployment WHERE id = $1")
		.bind(id)
		.fetch_one(&setup.database)
		.await
		.unwrap();
	let status: String = row.get("status");
	assert_eq!(status, "running");
}

#[tokio::test]
async fn stopped_from_executor_running_in_db_calls_upsert() {
	let (setup, id) = setup_with_statuses(
		DeploymentStatus::Running,
		DeploymentStatus::Stopped,
	)
	.await;

	// Actor should try to restart the deployment via upsert.
	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Upsert(i) if *i == id)),
		Duration::from_secs(5),
	)
	.await;
}

#[tokio::test]
async fn running_from_executor_stopped_in_db_calls_delete() {
	let (setup, id) = setup_with_statuses(
		DeploymentStatus::Stopped,
		DeploymentStatus::Running,
	)
	.await;

	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Delete(i) if *i == id)),
		Duration::from_secs(5),
	)
	.await;
}

// NOTE: Unreachable is not a valid DB status (CHECK constraint rejects it),
// so the (Unreachable, *) branch can only be triggered by the executor
// reporting Unreachable, not by the DB having it. That branch is covered
// by the actor seeing executor_status=Unreachable and updating the DB.

#[tokio::test]
async fn duplicate_status_not_re_reported() {
	let setup = setup().await;
	let id = setup
		.create_test_deployment_with_status(DeploymentStatus::Deploying)
		.await;

	// Mock reports Running — first CheckStatus should update DB and report.
	setup
		.mock_state
		.statuses
		.lock()
		.unwrap()
		.insert(id, DeploymentStatus::Running);

	setup.notify_upsert(id);

	// Wait for the initial upsert + first status check.
	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.call_count(|c| matches!(c, ExecutorCall::GetStatus(i) if *i == id)) >= 1,
		Duration::from_secs(10),
	)
	.await;

	// DB should now be Running.
	let row = sqlx::query("SELECT status FROM deployment WHERE id = $1")
		.bind(id)
		.fetch_one(&setup.database)
		.await
		.unwrap();
	let status: String = row.get("status");
	assert_eq!(status, "running");

	// Wait for a second status check — status is still Running, so
	// the actor should NOT send another ResourceStatusChanged (no-op).
	// We can't directly observe ResourceStatusChanged messages from here,
	// but we can verify no executor side-effects happen (no upsert/delete).
	setup.mock_state.calls.lock().unwrap().clear();
	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.call_count(|c| matches!(c, ExecutorCall::GetStatus(i) if *i == id)) >= 1,
		Duration::from_secs(10),
	)
	.await;

	assert!(
		!setup.mock_state.has_call(|c| matches!(c, ExecutorCall::Upsert(_))),
		"expected no upsert on duplicate status"
	);
	assert!(
		!setup.mock_state.has_call(|c| matches!(c, ExecutorCall::Delete(_))),
		"expected no delete on duplicate status"
	);
}

#[tokio::test]
async fn deploying_in_db_errored_from_executor_updates_db() {
	let (setup, id) = setup_with_statuses(
		DeploymentStatus::Deploying,
		DeploymentStatus::Errored,
	)
	.await;

	let row = sqlx::query("SELECT status FROM deployment WHERE id = $1")
		.bind(id)
		.fetch_one(&setup.database)
		.await
		.unwrap();
	let status: String = row.get("status");
	assert_eq!(status, "errored");
}
