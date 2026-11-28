use crate::prelude::*;

/// A Patr registry deployment keeps its variant through SQLite: the local state
/// holds a `PatrRegistry` with the same repository, never an external registry
/// that would be pulled without Patr's credentials.
#[tokio::test]
async fn patr_registry_deployment_round_trips() {
	let setup = setup().await;
	let id = Uuid::new_v4();
	let repository_id = Uuid::new_v4();

	let mut conn = setup.database.acquire().await.unwrap();
	common::runner::db_helpers::upsert_deployment_in_database(
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

	let (upserted, _) = common::runner::db_helpers::get_local_deployment_info(&setup.database, id)
		.await
		.unwrap();
	assert_eq!(
		upserted.registry,
		DeploymentRegistry::PatrRegistry {
			registry: PatrRegistry,
			repository_id,
		}
	);
	assert_eq!(upserted.current_live_digest.as_deref(), Some("sha256:abc"));
}

/// Upserts only ever write the desired columns: the observed status the
/// controller recorded survives a `DeploymentUpdated`, the way it survives a
/// resync.
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
	common::runner::db_helpers::upsert_deployment_in_database(
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
