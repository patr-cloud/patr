use crate::prelude::*;

#[tokio::test]
async fn spawn_deployment_actor_calls_upsert() {
	let setup = setup().await;
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
async fn config_updated_no_change_is_noop() {
	let setup = setup().await;
	let id = setup.create_test_deployment().await;

	setup.notify_upsert(id);

	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Upsert(i) if *i == id)),
		Duration::from_secs(5),
	)
	.await;

	// Clear calls, then send another UpsertResource (triggers ConfigUpdated).
	// Config hasn't changed in SQLite, so it should be a no-op.
	setup.mock_state.calls.lock().unwrap().clear();
	setup.notify_upsert(id);

	tokio::time::sleep(Duration::from_millis(500)).await;
	assert_eq!(
		setup
			.mock_state
			.call_count(|c| matches!(c, ExecutorCall::Upsert(_))),
		0,
		"expected no upsert call when config hasn't changed"
	);
}

#[tokio::test]
async fn config_updated_with_change_calls_upsert() {
	let setup = setup().await;
	let id = setup.create_test_deployment().await;

	setup.notify_upsert(id);

	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Upsert(i) if *i == id)),
		Duration::from_secs(5),
	)
	.await;

	// Change the deployment config in SQLite.
	sqlx::query("UPDATE deployment SET image_tag = 'v2' WHERE id = $1")
		.bind(id)
		.execute(&setup.database)
		.await
		.unwrap();

	// Clear calls and notify — actor should detect the change and upsert.
	setup.mock_state.calls.lock().unwrap().clear();
	setup.notify_upsert(id);

	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Upsert(i) if *i == id)),
		Duration::from_secs(5),
	)
	.await;
}

#[tokio::test]
async fn shutdown_running_deployment_calls_delete() {
	let setup = setup().await;
	let id = setup.create_test_deployment().await;

	// Mock reports this deployment as Running.
	setup
		.mock_state
		.statuses
		.lock()
		.unwrap()
		.insert(id, DeploymentStatus::Running);

	setup.notify_upsert(id);

	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Upsert(i) if *i == id)),
		Duration::from_secs(5),
	)
	.await;

	// Now send delete — supervisor sends Shutdown to the actor.
	setup.mock_state.calls.lock().unwrap().clear();
	setup.notify_delete(id);

	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Delete(i) if *i == id)),
		Duration::from_secs(5),
	)
	.await;
}

#[tokio::test]
async fn shutdown_stopped_deployment_does_not_call_delete() {
	let setup = setup().await;
	let id = setup.create_test_deployment().await;

	// Mock reports Stopped (the default).
	setup.notify_upsert(id);

	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Upsert(i) if *i == id)),
		Duration::from_secs(5),
	)
	.await;

	setup.mock_state.calls.lock().unwrap().clear();
	setup.notify_delete(id);

	// Wait a bit — delete should NOT be called since executor reports Stopped.
	tokio::time::sleep(Duration::from_millis(500)).await;
	assert!(
		!setup
			.mock_state
			.has_call(|c| matches!(c, ExecutorCall::Delete(_))),
		"expected no delete call for a stopped deployment"
	);
}

#[tokio::test]
async fn deployment_deleted_from_db_stops_actor() {
	let setup = setup().await;
	let id = setup.create_test_deployment().await;

	setup
		.mock_state
		.statuses
		.lock()
		.unwrap()
		.insert(id, DeploymentStatus::Running);

	setup.notify_upsert(id);

	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Upsert(i) if *i == id)),
		Duration::from_secs(5),
	)
	.await;

	// Delete from SQLite — the actor's next CheckStatus poll will find
	// it missing and call delete_deployment + stop itself.
	sqlx::query("DELETE FROM deployment WHERE id = $1")
		.bind(id)
		.execute(&setup.database)
		.await
		.unwrap();

	setup.mock_state.calls.lock().unwrap().clear();

	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Delete(i) if *i == id)),
		Duration::from_secs(10),
	)
	.await;
}
