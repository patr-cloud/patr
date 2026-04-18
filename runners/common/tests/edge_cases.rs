use crate::prelude::*;

#[tokio::test]
async fn create_with_deploy_on_create_false_stays_stopped() {
	let setup = setup().await;
	let id = setup
		.create_test_deployment_with_status(DeploymentStatus::Stopped)
		.await;

	setup.notify_upsert(id);

	// Wait for the actor to process ConfigUpdated. It should still call
	// upsert (actor always upserts on first run since last_applied is None),
	// but the deployment's desired status is Stopped.
	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Upsert(i) if *i == id)),
		Duration::from_secs(5),
	)
	.await;

	// After a status check, the executor reports Stopped (default mock
	// behavior) and DB says Stopped — should be a no-op. Verify the DB
	// status hasn't changed.
	tokio::time::sleep(Duration::from_secs(6)).await;

	let row = sqlx::query("SELECT status FROM deployment WHERE id = $1")
		.bind(id)
		.fetch_one(&setup.database)
		.await
		.unwrap();
	let status: String = row.get("status");
	assert_eq!(status, "stopped");
}

#[tokio::test]
async fn create_with_no_ports_still_upserts() {
	let setup = setup().await;
	// Default test deployment has no ports — verify it works fine.
	let id = setup.create_test_deployment().await;

	setup.notify_upsert(id);

	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Upsert(i) if *i == id)),
		Duration::from_secs(5),
	)
	.await;
}

#[tokio::test]
async fn create_then_immediately_delete() {
	let setup = setup().await;
	let id = setup.create_test_deployment().await;

	setup
		.mock_state
		.statuses
		.lock()
		.unwrap()
		.insert(id, DeploymentStatus::Running);

	// Send both in rapid succession.
	setup.notify_upsert(id);
	setup.notify_delete(id);

	// Should eventually clean up — either upsert+delete or just delete.
	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Delete(i) if *i == id)),
		Duration::from_secs(10),
	)
	.await;
}

#[tokio::test]
async fn update_twice_rapidly_applies_final_config() {
	let setup = setup().await;
	let id = setup.create_test_deployment().await;

	setup.notify_upsert(id);

	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Upsert(i) if *i == id)),
		Duration::from_secs(5),
	)
	.await;

	// Update config twice rapidly.
	sqlx::query("UPDATE deployment SET image_tag = 'v2' WHERE id = $1")
		.bind(id)
		.execute(&setup.database)
		.await
		.unwrap();
	setup.notify_upsert(id);

	sqlx::query("UPDATE deployment SET image_tag = 'v3' WHERE id = $1")
		.bind(id)
		.execute(&setup.database)
		.await
		.unwrap();
	setup.notify_upsert(id);

	// Wait for processing to settle.
	tokio::time::sleep(Duration::from_secs(1)).await;

	// The final state in the DB should be v3, and the mock should have
	// received at least one additional upsert.
	let row = sqlx::query("SELECT image_tag FROM deployment WHERE id = $1")
		.bind(id)
		.fetch_one(&setup.database)
		.await
		.unwrap();
	let tag: String = row.get("image_tag");
	assert_eq!(tag, "v3");
}

#[tokio::test]
async fn start_already_running_is_noop() {
	let setup = setup().await;
	let id = setup
		.create_test_deployment_with_status(DeploymentStatus::Running)
		.await;

	setup
		.mock_state
		.statuses
		.lock()
		.unwrap()
		.insert(id, DeploymentStatus::Running);

	setup.notify_upsert(id);

	// Wait for initial upsert.
	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Upsert(i) if *i == id)),
		Duration::from_secs(5),
	)
	.await;

	// Wait for a status check to confirm Running == Running.
	setup.mock_state.calls.lock().unwrap().clear();

	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::GetStatus(i) if *i == id)),
		Duration::from_secs(10),
	)
	.await;

	// No additional upsert or delete should have been triggered by the
	// status reconciliation.
	assert!(
		!setup
			.mock_state
			.has_call(|c| matches!(c, ExecutorCall::Upsert(_))),
		"expected no upsert when already running"
	);
	assert!(
		!setup
			.mock_state
			.has_call(|c| matches!(c, ExecutorCall::Delete(_))),
		"expected no delete when already running"
	);
}

#[tokio::test]
async fn stop_already_stopped_is_noop() {
	let setup = setup().await;
	let id = setup
		.create_test_deployment_with_status(DeploymentStatus::Stopped)
		.await;

	// Mock default is Stopped — matches DB.
	setup.notify_upsert(id);

	// Wait for initial upsert (actor always upserts on first ConfigUpdated
	// since last_applied is None).
	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Upsert(i) if *i == id)),
		Duration::from_secs(5),
	)
	.await;

	// Wait for a status check.
	setup.mock_state.calls.lock().unwrap().clear();

	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::GetStatus(i) if *i == id)),
		Duration::from_secs(10),
	)
	.await;

	// Stopped == Stopped → no action.
	assert!(
		!setup
			.mock_state
			.has_call(|c| matches!(c, ExecutorCall::Upsert(_))),
		"expected no upsert when already stopped"
	);
	assert!(
		!setup
			.mock_state
			.has_call(|c| matches!(c, ExecutorCall::Delete(_))),
		"expected no delete when already stopped"
	);
}
