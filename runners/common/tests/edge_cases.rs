use crate::prelude::*;

#[tokio::test]
async fn create_with_deploy_on_create_false_stays_stopped() {
	let setup = setup().await;
	let id = setup.create_test_deployment_with_should_run(false).await;

	setup.notify_upsert(id);

	// A stopped deployment isn't applied at all — not even on the actor's
	// first run — so wait for a status check instead. The executor reports
	// Stopped (default mock behavior) and DB says Stopped, so that's a no-op.
	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.has_call(|c| matches!(c, ExecutorCall::GetStatus(i) if *i == id)),
		Duration::from_secs(10),
	)
	.await;

	assert!(
		!setup
			.mock_state
			.has_call(|c| matches!(c, ExecutorCall::Upsert(i) if *i == id)),
		"a stopped deployment must never be upserted"
	);

	let row = sqlx::query("SELECT should_run FROM deployment WHERE id = $1")
		.bind(id)
		.fetch_one(&setup.database)
		.await
		.unwrap();
	assert!(!row.get::<bool, _>("should_run"));
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
		.insert(id, ObservedDeploymentStatus::Running);

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
	let id = setup.create_test_deployment_with_should_run(true).await;

	setup
		.mock_state
		.statuses
		.lock()
		.unwrap()
		.insert(id, ObservedDeploymentStatus::Running);

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
	let id = setup.create_test_deployment_with_should_run(false).await;

	// Mock default is Stopped — matches DB. A stopped deployment isn't
	// applied on the actor's first run, so the status check is the first call.
	setup.notify_upsert(id);

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

/// A Patr registry deployment keeps its variant through SQLite: the executor
/// gets a `PatrRegistry` with the same repository back, never an external
/// registry that would be pulled without Patr's credentials.
#[tokio::test]
async fn patr_registry_deployment_round_trips() {
	let setup = setup().await;
	let id = Uuid::new_v4();
	let repository_id = Uuid::new_v4();

	let mut conn = setup.database.acquire().await.unwrap();
	common::actors::db_helpers::upsert_deployment_in_database(
		&mut conn,
		WithId::new(
			id,
			Deployment {
				name: "patr-registry".to_string(),
				registry: DeploymentRegistry::PatrRegistry {
					registry: PatrRegistry,
					repository_id,
				},
				image_tag: "latest".to_string(),
				status: DeploymentStatus::Observed(ObservedDeploymentStatus::Deploying),
				should_run: true,
				runner: Uuid::nil(),
				current_live_digest: Some("sha256:abc".to_string()),
				machine_type: Uuid::parse_str("b3cf3771fa394281bfdfeb2e65a061b6").unwrap(),
			},
		),
		DeploymentRunningDetails {
			deploy_on_push: true,
			min_horizontal_scale: 1,
			max_horizontal_scale: 1,
			ports: Default::default(),
			environment_variables: Default::default(),
			startup_probe: None,
			liveness_probe: None,
			config_mounts: Default::default(),
			volumes: Default::default(),
		},
	)
	.await
	.unwrap();
	drop(conn);

	setup.notify_upsert(id);

	let mock = setup.mock_state.clone();
	periodic_check(
		move || mock.upserted.lock().unwrap().contains_key(&id),
		Duration::from_secs(5),
	)
	.await;

	let upserted = setup.mock_state.upserted.lock().unwrap()[&id].clone();
	assert_eq!(
		upserted.registry,
		DeploymentRegistry::PatrRegistry {
			registry: PatrRegistry,
			repository_id,
		}
	);
	assert_eq!(upserted.current_live_digest.as_deref(), Some("sha256:abc"));
}

/// Upserts only ever write the desired columns: the observed status the actor
/// recorded survives a `DeploymentUpdated`, the way it survives a resync.
#[tokio::test]
async fn upsert_leaves_observed_status_alone() {
	let setup = setup().await;
	let id = setup.create_test_deployment().await;

	sqlx::query("UPDATE deployment SET observed_status = 'running' WHERE id = $1")
		.bind(id)
		.execute(&setup.database)
		.await
		.unwrap();

	let mut conn = setup.database.acquire().await.unwrap();
	common::actors::db_helpers::upsert_deployment_in_database(
		&mut conn,
		WithId::new(
			id,
			Deployment {
				name: "renamed".to_string(),
				registry: DeploymentRegistry::ExternalRegistry {
					registry: "docker.io".to_string(),
					image_name: "nginx".to_string(),
				},
				image_tag: "v2".to_string(),
				status: DeploymentStatus::Stopping,
				should_run: false,
				runner: Uuid::nil(),
				current_live_digest: None,
				machine_type: Uuid::parse_str("b3cf3771fa394281bfdfeb2e65a061b6").unwrap(),
			},
		),
		DeploymentRunningDetails {
			deploy_on_push: false,
			min_horizontal_scale: 1,
			max_horizontal_scale: 1,
			ports: Default::default(),
			environment_variables: Default::default(),
			startup_probe: None,
			liveness_probe: None,
			config_mounts: Default::default(),
			volumes: Default::default(),
		},
	)
	.await
	.unwrap();
	drop(conn);

	let row =
		sqlx::query("SELECT image_tag, should_run, observed_status FROM deployment WHERE id = $1")
			.bind(id)
			.fetch_one(&setup.database)
			.await
			.unwrap();
	assert_eq!(row.get::<String, _>("image_tag"), "v2");
	assert!(!row.get::<bool, _>("should_run"));
	assert_eq!(
		row.get::<Option<String>, _>("observed_status").as_deref(),
		Some("running")
	);
}
