use common::runner::{
	controller::{self, ControllerMessage},
	observer::{self, ObserverMessage},
};
use tokio::sync::mpsc;

use crate::prelude::*;

/// Runs a controller on the setup's database, returning the sender for its
/// messages.
fn spawn_controller(setup: &TestSetup) -> mpsc::UnboundedSender<ControllerMessage> {
	let (controller_tx, controller_rx) = mpsc::unbounded_channel();
	let (observer_tx, _) = mpsc::unbounded_channel();
	tokio::spawn(controller::run(
		controller_rx,
		observer_tx,
		setup.executor.clone(),
		setup.config.clone(),
		setup.database.clone(),
	));
	controller_tx
}

/// Enqueued owners are reconciled.
#[tokio::test]
async fn enqueued_owners_are_reconciled() {
	let setup = setup().await;
	let id = setup.create_test_deployment().await;
	let controller = spawn_controller(&setup);

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

/// The observer records what the backend reports, a deployment the backend
/// doesn't run as stopped, and a deployment whose objects failed to apply as
/// errored whatever is still running.
#[tokio::test]
async fn the_observer_records_observed_statuses() {
	let setup = setup().await;
	let running = setup.create_test_deployment().await;
	let missing = setup.create_test_deployment().await;
	setup
		.mock_state
		.statuses
		.lock()
		.unwrap()
		.insert(running, ObservedDeploymentStatus::Running);

	let (observer_tx, observer_rx) = mpsc::unbounded_channel();
	let (controller_tx, _) = mpsc::unbounded_channel();
	let (websocket_tx, _) = mpsc::unbounded_channel();
	tokio::spawn(observer::run(
		observer_rx,
		controller_tx,
		websocket_tx,
		setup.executor.clone(),
		setup.database.clone(),
	));

	wait_for_deployment_status(&setup.database, running, "running", Duration::from_secs(5)).await;
	wait_for_deployment_status(&setup.database, missing, "stopped", Duration::from_secs(5)).await;

	observer_tx
		.send(ObserverMessage::ApplyOutcome {
			deployment_id: running,
			failed: true,
		})
		.unwrap();
	wait_for_deployment_status(&setup.database, running, "errored", Duration::from_secs(5)).await;
}
