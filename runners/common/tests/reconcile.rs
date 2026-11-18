use std::collections::BTreeMap;

use crate::prelude::*;

/// A deployment that should run, with a volume per path and a config mount
/// per `(path, content)`.
fn deployment(
	should_run: bool,
	volumes: &[&str],
	config_mounts: &[(&str, &str)],
) -> (Deployment, DeploymentRunningDetails) {
	(
		Deployment {
			name: "reconcile".to_string(),
			registry: DeploymentRegistry::ExternalRegistry {
				registry: "docker.io".to_string(),
				image_name: "nginx".to_string(),
			},
			image_tag: "latest".to_string(),
			status: DeploymentStatus::Observed(ObservedDeploymentStatus::Deploying),
			should_run,
			runner: Uuid::nil(),
			current_live_digest: None,
			machine_type: Uuid::parse_str("b3cf3771fa394281bfdfeb2e65a061b6").unwrap(),
		},
		DeploymentRunningDetails {
			deploy_on_push: false,
			min_horizontal_scale: 1,
			max_horizontal_scale: 1,
			ports: BTreeMap::new(),
			environment_variables: BTreeMap::new(),
			startup_probe: None,
			liveness_probe: None,
			config_mounts: config_mounts
				.iter()
				.map(|(path, content)| {
					(
						path.to_string(),
						Base64String::from(content.as_bytes().to_vec()),
					)
				})
				.collect(),
			volumes: volumes
				.iter()
				.map(|path| (path.to_string(), VolumeConfig {}))
				.collect(),
		},
	)
}

/// The position of a call in the log, for asserting order.
fn position(state: &MockExecutorState, call: ExecutorCall) -> usize {
	state
		.calls
		.lock()
		.unwrap()
		.iter()
		.position(|c| *c == call)
		.unwrap_or_else(|| panic!("{call:?} was never made"))
}

/// Dependencies are applied before what depends on them, and a second
/// reconcile with nothing changed applies nothing.
#[tokio::test]
async fn applies_dependencies_first_and_only_once() {
	let setup = setup().await;
	let id = Uuid::new_v4();
	let (deployment, details) = deployment(true, &["/data"], &[("/etc/app.conf", "a")]);
	setup.upsert_deployment(id, deployment, details).await;

	let outcome = setup.reconcile(Owner::Deployment(id)).await;
	assert!(outcome.changed && !outcome.failed && !outcome.waiting);

	let mock = &setup.mock_state;
	let service_applied = position(mock, ExecutorCall::Apply(service(id)));
	assert!(position(mock, ExecutorCall::Apply(volume(id, "/data"))) < service_applied);
	assert!(position(mock, ExecutorCall::Apply(config(id, "/etc/app.conf"))) < service_applied);
	assert!(mock.exists(&routes(id)));

	let calls = mock.calls.lock().unwrap().len();
	let outcome = setup.reconcile(Owner::Deployment(id)).await;
	assert!(!outcome.changed);
	assert_eq!(mock.calls.lock().unwrap().len(), calls);
}

/// A changed dependency re-applies everything that depends on it, and nothing
/// else.
#[tokio::test]
async fn a_changed_dependency_reapplies_its_dependents() {
	let setup = setup().await;
	let id = Uuid::new_v4();
	let (deployment_v1, details) = deployment(true, &["/data"], &[("/etc/app.conf", "a")]);
	setup.upsert_deployment(id, deployment_v1, details).await;
	setup.reconcile(Owner::Deployment(id)).await;
	setup.mock_state.calls.lock().unwrap().clear();

	let (deployment_v2, details) = deployment(true, &["/data"], &[("/etc/app.conf", "b")]);
	setup.upsert_deployment(id, deployment_v2, details).await;
	setup.reconcile(Owner::Deployment(id)).await;

	let mock = &setup.mock_state;
	assert!(mock.has_call(|c| *c == ExecutorCall::Apply(config(id, "/etc/app.conf"))));
	assert!(mock.has_call(|c| *c == ExecutorCall::Apply(service(id))));
	assert!(!mock.has_call(|c| *c == ExecutorCall::Apply(volume(id, "/data"))));
}

/// An object that fails to apply blocks what depends on it, but not its
/// siblings.
#[tokio::test]
async fn a_failed_object_blocks_only_its_dependents() {
	let setup = setup().await;
	let id = Uuid::new_v4();
	let (deployment, details) = deployment(true, &["/data"], &[("/etc/app.conf", "a")]);
	setup.upsert_deployment(id, deployment, details).await;
	setup
		.mock_state
		.apply_errors
		.lock()
		.unwrap()
		.insert(config(id, "/etc/app.conf"));

	let outcome = setup.reconcile(Owner::Deployment(id)).await;
	assert!(outcome.failed);

	let mock = &setup.mock_state;
	assert!(mock.exists(&volume(id, "/data")));
	assert!(mock.exists(&routes(id)));
	assert!(!mock.has_call(|c| *c == ExecutorCall::Apply(service(id))));
}

/// Stopping a deployment deletes its service before the configs it mounted,
/// and keeps its volumes and routes.
#[tokio::test]
async fn stopping_reaps_dependents_first_and_keeps_volumes() {
	let setup = setup().await;
	let id = Uuid::new_v4();
	let (running, details) = deployment(true, &["/data"], &[("/etc/app.conf", "a")]);
	setup.upsert_deployment(id, running, details).await;
	setup.reconcile(Owner::Deployment(id)).await;

	let (stopped, details) = deployment(false, &["/data"], &[("/etc/app.conf", "a")]);
	setup.upsert_deployment(id, stopped, details).await;
	let outcome = setup.reconcile(Owner::Deployment(id)).await;
	assert!(outcome.changed && !outcome.waiting);

	let mock = &setup.mock_state;
	assert!(
		position(mock, ExecutorCall::Delete(service(id))) <
			position(mock, ExecutorCall::Delete(config(id, "/etc/app.conf")))
	);
	assert!(mock.exists(&volume(id, "/data")));
	assert_eq!(
		mock.spec(&routes(id)).as_deref(),
		Some("routes should_run=false")
	);
}

/// An object that's still in use isn't deleted, and the reconcile waits.
#[tokio::test]
async fn an_object_in_use_is_left_for_later() {
	let setup = setup().await;
	let id = Uuid::new_v4();
	let (running, details) = deployment(true, &[], &[("/etc/app.conf", "a")]);
	setup.upsert_deployment(id, running, details).await;
	setup.reconcile(Owner::Deployment(id)).await;

	let (stopped, details) = deployment(false, &[], &[("/etc/app.conf", "a")]);
	setup.upsert_deployment(id, stopped, details).await;
	setup
		.mock_state
		.in_use
		.lock()
		.unwrap()
		.insert(config(id, "/etc/app.conf"));
	let outcome = setup.reconcile(Owner::Deployment(id)).await;

	assert!(outcome.waiting);
	assert!(setup.mock_state.exists(&config(id, "/etc/app.conf")));
	assert!(!setup.mock_state.exists(&service(id)));
}

/// A managed URL follows its deployment: its route answers as stopped when the
/// deployment is stopped, and goes away when the deployment does.
#[tokio::test]
async fn a_managed_url_follows_its_deployment() {
	let setup = setup().await;
	let id = Uuid::new_v4();
	let managed_url_id = Uuid::new_v4();
	let (running, details) = deployment(true, &[], &[]);
	setup.upsert_deployment(id, running, details).await;

	let mut conn = setup.database.acquire().await.unwrap();
	common::actors::db_helpers::upsert_managed_url_in_database(
		&mut conn,
		managed_url_id,
		"app.example.com",
		"/",
		id,
		8080,
	)
	.await
	.unwrap();
	drop(conn);

	setup.reconcile(Owner::ManagedUrl(managed_url_id)).await;
	assert_eq!(
		setup.mock_state.spec(&url(managed_url_id)).as_deref(),
		Some("url app.example.com/ should_run=true")
	);

	let (stopped, details) = deployment(false, &[], &[]);
	setup.upsert_deployment(id, stopped, details).await;
	setup.reconcile(Owner::ManagedUrl(managed_url_id)).await;
	assert_eq!(
		setup.mock_state.spec(&url(managed_url_id)).as_deref(),
		Some("url app.example.com/ should_run=false")
	);

	setup.delete_deployment(id).await;
	setup.reconcile(Owner::ManagedUrl(managed_url_id)).await;
	assert!(!setup.mock_state.exists(&url(managed_url_id)));
}
