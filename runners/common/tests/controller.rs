use common::runner::{
	controller::{self, ControllerMessage},
	websocket::WebSocketMessage,
};
use tokio::sync::mpsc;

use crate::prelude::*;

/// Runs a controller on the setup's database, returning the sender for its
/// messages and the receiver for the statuses it sends upstream.
fn spawn_controller(
	setup: &TestSetup,
) -> (
	mpsc::UnboundedSender<ControllerMessage>,
	mpsc::UnboundedReceiver<WebSocketMessage>,
) {
	let (controller_tx, controller_rx) = mpsc::unbounded_channel();
	let (websocket_tx, websocket_rx) = mpsc::unbounded_channel();
	tokio::spawn(controller::run(
		controller_rx,
		websocket_tx,
		setup.executor.clone(),
		setup.config.clone(),
		setup.database.clone(),
	));
	(controller_tx, websocket_rx)
}

/// How many times the backend was asked for a deployment's status.
fn observations(setup: &TestSetup, deployment_id: Uuid) -> usize {
	setup
		.mock_state
		.call_count(|c| *c == ExecutorCall::Observe(deployment_id))
}

/// Waits until the backend was asked for a deployment's status `count` times.
async fn wait_for_observations(setup: &TestSetup, deployment_id: Uuid, count: usize) {
	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.call_count(|c| *c == ExecutorCall::Observe(deployment_id)) >= count,
		Duration::from_secs(5),
	)
	.await;
}

/// Waits until the controller watches the backend's events.
async fn wait_for_watch(setup: &TestSetup) {
	let mock = setup.mock_state.clone();
	periodic_check(
		move || !mock.watchers.lock().unwrap().is_empty(),
		Duration::from_secs(5),
	)
	.await;
}

/// Enqueued owners are reconciled.
#[tokio::test]
async fn enqueued_owners_are_reconciled() {
	let setup = setup().await;
	let id = setup.create_test_deployment().await;
	let (controller, _websocket) = spawn_controller(&setup);

	controller
		.send(ControllerMessage::Enqueue([Owner::Deployment(id)].into()))
		.unwrap();

	let mock = setup.mock_state.clone();
	periodic_check(move || mock.exists(&service(id)), Duration::from_secs(5)).await;
}

/// A finalizer left marked running by a crash is reset when the controller
/// starts, so it runs again.
#[tokio::test]
async fn an_interrupted_finalizer_is_reset_on_start() {
	let setup = setup().await;
	sqlx::query(
		"INSERT INTO object_finalizer(object_id, deployment_id, finalizer, state, attempts, last_updated) VALUES ('volume/x', $1, 'volume', 'running', 1, $2)",
	)
	.bind(Uuid::new_v4())
	.bind(time::OffsetDateTime::now_utc())
	.execute(&setup.database)
	.await
	.unwrap();

	let _controller = spawn_controller(&setup);

	wait_until(
		async || {
			sqlx::query("SELECT state FROM object_finalizer WHERE object_id = 'volume/x'")
				.fetch_one(&setup.database)
				.await
				.unwrap()
				.get::<String, _>("state") ==
				"pending"
		},
		Duration::from_secs(5),
	)
	.await;
}

/// A reconcile records its deployment's observed status, what the backend
/// reports or stopped if it runs nothing of it, and sends it upstream once.
#[tokio::test]
async fn reconciles_record_and_send_observed_statuses() {
	let setup = setup().await;
	let running = setup.create_test_deployment().await;
	let missing = setup.create_test_deployment().await;
	setup
		.mock_state
		.statuses
		.lock()
		.unwrap()
		.insert(running, ObservedDeploymentStatus::Running);
	let (controller, mut websocket) = spawn_controller(&setup);

	wait_for_deployment_status(&setup.database, running, "running", Duration::from_secs(5)).await;
	wait_for_deployment_status(&setup.database, missing, "stopped", Duration::from_secs(5)).await;

	// Reconciling it again changes nothing, so nothing more is sent.
	controller
		.send(ControllerMessage::Enqueue(
			[Owner::Deployment(running)].into(),
		))
		.unwrap();
	wait_for_observations(&setup, running, 2).await;
	tokio::time::sleep(Duration::from_millis(200)).await;

	let mut sent = Vec::new();
	while let Ok(WebSocketMessage::SendStatusUpdate {
		resource_id,
		status,
	}) = websocket.try_recv()
	{
		sent.push((resource_id, status));
	}
	assert_eq!(sent.len(), 2, "sent {sent:?}");
	assert!(sent.contains(&(running, ObservedDeploymentStatus::Running)));
	assert!(sent.contains(&(missing, ObservedDeploymentStatus::Stopped)));
}

/// A deployment whose objects failed to apply reads errored, whatever the
/// backend runs of it.
#[tokio::test]
async fn a_failed_apply_records_errored() {
	let setup = setup().await;
	let id = setup.create_test_deployment().await;
	setup
		.mock_state
		.statuses
		.lock()
		.unwrap()
		.insert(id, ObservedDeploymentStatus::Running);
	setup
		.mock_state
		.apply_errors
		.lock()
		.unwrap()
		.insert(service(id));

	let _controller = spawn_controller(&setup);

	wait_for_deployment_status(&setup.database, id, "errored", Duration::from_secs(5)).await;
}

/// A deployment that's still deploying is observed again shortly, since the
/// backend may report nothing when it fails to deploy.
#[tokio::test]
async fn a_deploying_deployment_is_observed_again() {
	let setup = setup().await;
	let id = setup.create_test_deployment().await;
	setup
		.mock_state
		.statuses
		.lock()
		.unwrap()
		.insert(id, ObservedDeploymentStatus::Deploying);
	let _controller = spawn_controller(&setup);
	wait_for_deployment_status(&setup.database, id, "deploying", Duration::from_secs(5)).await;

	// No event reports this, as when an image fails to pull.
	setup
		.mock_state
		.statuses
		.lock()
		.unwrap()
		.insert(id, ObservedDeploymentStatus::ImagePullFailed);

	// Sooner than the next full reconcile.
	wait_for_deployment_status(
		&setup.database,
		id,
		"image_pull_failed",
		Duration::from_secs(8),
	)
	.await;
}

/// A burst of events while an owner is being reconciled runs it once more, not
/// once per event.
#[tokio::test]
async fn a_burst_of_events_reconciles_once_more() {
	let setup = setup().await;
	let id = setup.create_test_deployment().await;
	setup
		.mock_state
		.statuses
		.lock()
		.unwrap()
		.insert(id, ObservedDeploymentStatus::Running);
	let _controller = spawn_controller(&setup);
	wait_for_watch(&setup).await;
	wait_for_deployment_status(&setup.database, id, "running", Duration::from_secs(5)).await;
	tokio::time::sleep(Duration::from_millis(100)).await;
	let observed = observations(&setup, id);

	for _ in 0..10 {
		setup.mock_state.emit(Owner::Deployment(id));
	}

	wait_for_observations(&setup, id, observed + 2).await;
	tokio::time::sleep(Duration::from_millis(500)).await;
	assert_eq!(observations(&setup, id), observed + 2);
}

/// An event for an owner whose objects failed to apply waits for its retry,
/// since the owner reads errored until then whatever the backend says.
#[tokio::test]
async fn events_wait_for_an_owners_retry() {
	let setup = setup().await;
	let id = setup.create_test_deployment().await;
	setup
		.mock_state
		.apply_errors
		.lock()
		.unwrap()
		.insert(service(id));
	let _controller = spawn_controller(&setup);
	wait_for_watch(&setup).await;
	wait_for_deployment_status(&setup.database, id, "errored", Duration::from_secs(5)).await;
	tokio::time::sleep(Duration::from_millis(100)).await;
	let applies =
		|mock: &MockExecutorState| mock.call_count(|c| *c == ExecutorCall::Apply(service(id)));
	let applied = applies(&setup.mock_state);

	setup.mock_state.emit(Owner::Deployment(id));
	tokio::time::sleep(Duration::from_millis(300)).await;
	assert_eq!(applies(&setup.mock_state), applied);

	// The retry still runs, a second after the failure.
	let mock = setup.mock_state.clone();
	periodic_check(move || applies(&mock) > applied, Duration::from_secs(5)).await;
}

/// An upstream change reconciles an owner straight away, and replaces its
/// pending retry.
#[tokio::test]
async fn an_upstream_change_replaces_a_pending_retry() {
	let setup = setup().await;
	let id = setup.create_test_deployment().await;
	setup
		.mock_state
		.statuses
		.lock()
		.unwrap()
		.insert(id, ObservedDeploymentStatus::Running);
	setup
		.mock_state
		.apply_errors
		.lock()
		.unwrap()
		.insert(service(id));
	let (controller, _websocket) = spawn_controller(&setup);
	wait_for_deployment_status(&setup.database, id, "errored", Duration::from_secs(5)).await;
	tokio::time::sleep(Duration::from_millis(100)).await;

	setup.mock_state.apply_errors.lock().unwrap().clear();
	controller
		.send(ControllerMessage::Enqueue([Owner::Deployment(id)].into()))
		.unwrap();
	wait_for_deployment_status(&setup.database, id, "running", Duration::from_secs(5)).await;
	let observed = observations(&setup, id);

	// The retry was due a second after the failure.
	tokio::time::sleep(Duration::from_secs(2)).await;
	assert_eq!(observations(&setup, id), observed);
}

/// The backend's event stream ending reconciles every owner, since its events
/// may have been missed, and the backend is watched again.
#[tokio::test]
async fn the_watch_ending_reconciles_every_owner() {
	let setup = setup().await;
	let id = setup.create_test_deployment().await;
	setup
		.mock_state
		.statuses
		.lock()
		.unwrap()
		.insert(id, ObservedDeploymentStatus::Running);
	let _controller = spawn_controller(&setup);
	wait_for_watch(&setup).await;
	wait_for_deployment_status(&setup.database, id, "running", Duration::from_secs(5)).await;
	tokio::time::sleep(Duration::from_millis(100)).await;
	let observed = observations(&setup, id);

	setup.mock_state.watchers.lock().unwrap().clear();

	wait_for_observations(&setup, id, observed + 1).await;
	wait_for_watch(&setup).await;
}
