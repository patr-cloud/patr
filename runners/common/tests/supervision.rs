use crate::prelude::*;

#[tokio::test]
async fn failed_actor_is_respawned_with_backoff() {
	let setup = setup().await;
	let id = setup.create_test_deployment().await;

	// Configure upsert to fail — actor will crash on ConfigUpdated.
	setup
		.mock_state
		.upsert_errors
		.lock()
		.unwrap()
		.insert(id, "test failure".to_string());

	setup.notify_upsert(id);

	// Wait for the first failed upsert attempt.
	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Upsert(i) if *i == id)),
		Duration::from_secs(5),
	)
	.await;

	// The actor should crash and be respawned after a 1s backoff.
	// Wait for a second upsert attempt (the respawn).
	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.call_count(|c| matches!(c, ExecutorCall::Upsert(i) if *i == id)) >= 2,
		Duration::from_secs(10),
	)
	.await;

	// Verify it's not an immediate tight loop — there should be a gap
	// between attempts. With 1s initial backoff, 2 attempts in 10s is fine
	// but 10+ attempts would indicate no backoff.
	let count = setup
		.mock_state
		.call_count(|c| matches!(c, ExecutorCall::Upsert(i) if *i == id));
	assert!(
		count <= 5,
		"expected backoff to limit retries, got {count} upsert attempts"
	);
}

#[tokio::test]
async fn backoff_cleared_on_explicit_upsert() {
	let setup = setup().await;
	let id = setup.create_test_deployment().await;

	// Configure upsert to fail.
	setup
		.mock_state
		.upsert_errors
		.lock()
		.unwrap()
		.insert(id, "test failure".to_string());

	setup.notify_upsert(id);

	// Wait for the first failure.
	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Upsert(i) if *i == id)),
		Duration::from_secs(5),
	)
	.await;

	// Now fix the error and send an explicit UpsertResource — should
	// clear the backoff and retry immediately.
	setup.mock_state.upsert_errors.lock().unwrap().remove(&id);
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
async fn reconcile_skips_backed_off_deployments() {
	let setup = setup().await;
	let id = setup.create_test_deployment().await;

	// Configure upsert to always fail.
	setup
		.mock_state
		.upsert_errors
		.lock()
		.unwrap()
		.insert(id, "test failure".to_string());

	// Don't use notify_upsert — let Reconcile (10s in debug mode)
	// discover the deployment and spawn the actor naturally.
	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Upsert(i) if *i == id)),
		Duration::from_secs(15),
	)
	.await;

	// Now the deployment is in backoff. Wait for another full Reconcile
	// cycle. With exponential backoff (1s, 2s, 4s, ...), the retry count
	// should be bounded — not every 10s from Reconcile.
	let count_before = setup
		.mock_state
		.call_count(|c| matches!(c, ExecutorCall::Upsert(i) if *i == id));

	tokio::time::sleep(Duration::from_secs(12)).await;

	let count_after = setup
		.mock_state
		.call_count(|c| matches!(c, ExecutorCall::Upsert(i) if *i == id));

	// Backoff doubles: 1s → 2s → 4s → 8s. In 12 seconds, we'd expect
	// ~3-4 retries from the backoff timer. Without backoff, Reconcile
	// alone would cause ~1 retry per 10s plus immediate respawns.
	// Allow up to 5 to account for timing jitter.
	let delta = count_after - count_before;
	assert!(
		delta <= 5,
		"expected backoff to limit retries, but {delta} upsert attempts in 12s (from {count_before} to {count_after})"
	);
}

#[tokio::test]
async fn reconcile_spawns_missing_actors() {
	let setup = setup().await;
	let id = setup.create_test_deployment().await;

	// Don't send UpsertResource — just wait for periodic Reconcile (10s in
	// debug mode) to discover the deployment in SQLite and spawn an actor.
	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Upsert(i) if *i == id)),
		Duration::from_secs(15),
	)
	.await;
}

#[tokio::test]
async fn reconcile_stops_orphaned_actors() {
	let setup = setup().await;
	let id = setup.create_test_deployment().await;

	setup
		.mock_state
		.statuses
		.lock()
		.unwrap()
		.insert(id, DeploymentStatus::Running);

	setup.notify_upsert(id);

	// Wait for actor to be running.
	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Upsert(i) if *i == id)),
		Duration::from_secs(5),
	)
	.await;

	// Delete from SQLite — the actor becomes orphaned.
	// Either CheckStatus detects it or Reconcile stops it.
	sqlx::query("DELETE FROM deployment WHERE id = $1")
		.bind(id)
		.execute(&setup.database)
		.await
		.unwrap();

	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::Delete(i) if *i == id)),
		Duration::from_secs(15),
	)
	.await;
}
