use crate::prelude::*;

/// Helper: set up a deployment with the given intent and mock executor status,
/// then wait for the actor to process at least one CheckStatus cycle.
async fn setup_with_statuses(
	should_run: bool,
	executor_status: ObservedDeploymentStatus,
) -> (TestSetup, Uuid) {
	let setup = setup().await;
	let id = setup
		.create_test_deployment_with_should_run(should_run)
		.await;

	setup
		.mock_state
		.statuses
		.lock()
		.unwrap()
		.insert(id, executor_status);

	setup.notify_upsert(id);

	// Wait for the actor's first run to finish: an upsert, or — for a stopped
	// deployment, which is never applied — its first status check.
	let mock = setup.mock_state.clone();
	periodic_check(
		move || {
			mock.has_call(
				|c| matches!(c, ExecutorCall::Upsert(i) | ExecutorCall::GetStatus(i) if *i == id),
			)
		},
		Duration::from_secs(10),
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
async fn running_and_should_run_is_noop() {
	let (setup, id) = setup_with_statuses(true, ObservedDeploymentStatus::Running).await;

	// No upsert or delete should have been called after the status check.
	assert!(
		!setup
			.mock_state
			.has_call(|c| matches!(c, ExecutorCall::Upsert(i) if *i == id)),
		"expected no upsert when the deployment runs as it should"
	);
	assert!(
		!setup
			.mock_state
			.has_call(|c| matches!(c, ExecutorCall::Delete(i) | ExecutorCall::Stop(i) if *i == id)),
		"expected no delete or stop when the deployment runs as it should"
	);
}

#[tokio::test]
async fn observed_status_is_written_to_the_deployment() {
	let (setup, id) = setup_with_statuses(true, ObservedDeploymentStatus::Running).await;

	wait_for_deployment_status(&setup.database, id, "running", Duration::from_secs(5)).await;

	// Observed status never lands in the intent.
	let should_run = sqlx::query("SELECT should_run FROM deployment WHERE id = $1")
		.bind(id)
		.fetch_one(&setup.database)
		.await
		.unwrap()
		.get::<bool, _>("should_run");
	assert!(should_run);
}

#[tokio::test]
async fn errored_from_executor_is_written_to_the_deployment() {
	let (setup, id) = setup_with_statuses(true, ObservedDeploymentStatus::Errored).await;

	wait_for_deployment_status(&setup.database, id, "errored", Duration::from_secs(5)).await;
}

#[tokio::test]
async fn stopped_from_executor_and_should_run_calls_upsert() {
	let (setup, id) = setup_with_statuses(true, ObservedDeploymentStatus::Stopped).await;

	// Actor should try to restart the deployment via upsert.
	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Upsert(i) if *i == id)),
		Duration::from_secs(5),
	)
	.await;
}

#[tokio::test]
async fn running_from_executor_and_should_not_run_calls_stop_not_delete() {
	let (setup, id) = setup_with_statuses(false, ObservedDeploymentStatus::Running).await;

	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Stop(i) if *i == id)),
		Duration::from_secs(5),
	)
	.await;

	// A user pressing Stop must never reclaim the deployment's volumes.
	assert!(
		!setup
			.mock_state
			.has_call(|c| matches!(c, ExecutorCall::Delete(_))),
		"stop must route through stop_deployment, not delete_deployment"
	);
}

#[tokio::test]
async fn crashing_from_executor_and_should_not_run_calls_stop() {
	let (setup, id) = setup_with_statuses(false, ObservedDeploymentStatus::Crashing).await;

	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Stop(i) if *i == id)),
		Duration::from_secs(5),
	)
	.await;
}

#[tokio::test]
async fn duplicate_status_not_re_reported() {
	let setup = setup().await;
	let id = setup.create_test_deployment().await;

	// Mock reports Running — first CheckStatus should write and report it.
	setup
		.mock_state
		.statuses
		.lock()
		.unwrap()
		.insert(id, ObservedDeploymentStatus::Running);

	setup.notify_upsert(id);

	// Wait for the initial upsert + first status check.
	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.call_count(|c| matches!(c, ExecutorCall::GetStatus(i) if *i == id)) >= 1,
		Duration::from_secs(10),
	)
	.await;

	wait_for_deployment_status(&setup.database, id, "running", Duration::from_secs(5)).await;

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
		!setup
			.mock_state
			.has_call(|c| matches!(c, ExecutorCall::Upsert(_))),
		"expected no upsert on duplicate status"
	);
	assert!(
		!setup
			.mock_state
			.has_call(|c| matches!(c, ExecutorCall::Delete(_))),
		"expected no delete on duplicate status"
	);
}
