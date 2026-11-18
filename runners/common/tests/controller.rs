use common::actors::{
	controller::{Controller, ControllerArgs, ControllerMessage},
	observer::{Observer, ObserverArgs, ObserverMessage},
};
use ractor::Actor;

use crate::prelude::*;

/// A controller on a freshly created database reconciles nothing until the
/// first resync says the local state is complete.
#[tokio::test]
async fn a_fresh_database_waits_for_the_first_resync() {
	let setup = setup().await;
	let (controller, _handle) = Controller::<MockExecutor>::spawn(
		None,
		Controller::new(),
		ControllerArgs {
			executor: setup.executor.clone(),
			config: setup.config.clone(),
			database: setup.database.clone(),
			started: false,
		},
	)
	.await
	.unwrap();

	tokio::time::sleep(Duration::from_millis(300)).await;
	assert!(setup.mock_state.calls.lock().unwrap().is_empty());

	controller.send_message(ControllerMessage::Start).unwrap();

	let mock = setup.mock_state.clone();
	periodic_check(move || mock.exists(INGRESS), Duration::from_secs(5)).await;
}

/// Enqueued owners are reconciled once the controller has started.
#[tokio::test]
async fn enqueued_owners_are_reconciled() {
	let setup = setup().await;
	let id = setup.create_test_deployment().await;
	let (controller, _handle) = Controller::<MockExecutor>::spawn(
		None,
		Controller::new(),
		ControllerArgs {
			executor: setup.executor.clone(),
			config: setup.config.clone(),
			database: setup.database.clone(),
			started: true,
		},
	)
	.await
	.unwrap();

	controller
		.send_message(ControllerMessage::Enqueue([Owner::Deployment(id)].into()))
		.unwrap();

	let mock = setup.mock_state.clone();
	periodic_check(move || mock.exists(&service(id)), Duration::from_secs(5)).await;
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

	let (observer, _handle) = Observer::<MockExecutor>::spawn(
		None,
		Observer::new(),
		ObserverArgs {
			executor: setup.executor.clone(),
			database: setup.database.clone(),
		},
	)
	.await
	.unwrap();

	wait_for_deployment_status(&setup.database, running, "running", Duration::from_secs(5)).await;
	wait_for_deployment_status(&setup.database, missing, "stopped", Duration::from_secs(5)).await;

	observer
		.send_message(ObserverMessage::ApplyOutcome {
			deployment_id: running,
			failed: true,
		})
		.unwrap();
	wait_for_deployment_status(&setup.database, running, "errored", Duration::from_secs(5)).await;
}
