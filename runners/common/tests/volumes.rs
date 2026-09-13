//! Volumes on the runner's SQLite: the desired-state round trip.

use std::collections::BTreeMap;

use common::actors::db_helpers;

use crate::prelude::*;

/// Running details with the given volume paths and nothing else.
fn running_details_with_volumes(paths: &[&str]) -> DeploymentRunningDetails {
	DeploymentRunningDetails {
		deploy_on_push: false,
		min_horizontal_scale: 1,
		max_horizontal_scale: 1,
		ports: BTreeMap::new(),
		environment_variables: BTreeMap::new(),
		startup_probe: None,
		liveness_probe: None,
		config_mounts: BTreeMap::new(),
		volumes: paths
			.iter()
			.map(|path| ((*path).to_string(), VolumeConfig {}))
			.collect(),
	}
}

/// The deployment fixture used by every test here.
fn test_deployment() -> Deployment {
	Deployment {
		name: "volumes".to_string(),
		registry: DeploymentRegistry::ExternalRegistry {
			registry: "docker.io".to_string(),
			image_name: "nginx".to_string(),
		},
		image_tag: "latest".to_string(),
		status: DeploymentStatus::Running,
		runner: Uuid::nil(),
		current_live_digest: None,
		machine_type: Uuid::parse_str("b3cf3771fa394281bfdfeb2e65a061b6").unwrap(),
	}
}

/// The volume paths currently stored for a deployment, sorted.
async fn stored_volumes(database: &sqlx::Pool<DatabaseType>, id: Uuid) -> Vec<String> {
	sqlx::query("SELECT path FROM deployment_volume WHERE deployment_id = $1 ORDER BY path")
		.bind(id)
		.fetch_all(database)
		.await
		.unwrap()
		.into_iter()
		.map(|row| row.get::<String, _>("path"))
		.collect()
}

/// Regression: a deployment carrying a volume used to FK-violate on insert
/// (the old mount table pointed at a volume table nothing ever populated),
/// which rolled back the upstream transaction and wedged the WebSocket actor
/// in a retry loop. The upsert must commit, and must round-trip.
#[tokio::test]
async fn upsert_with_volumes_commits_and_round_trips() {
	let setup = setup().await;
	let id = Uuid::new_v4();
	let mut conn = setup.database.acquire().await.unwrap();

	db_helpers::upsert_deployment_in_database(
		&mut conn,
		WithId::new(id, test_deployment()),
		running_details_with_volumes(&["/data", "/var/lib/postgresql"]),
	)
	.await
	.expect("upsert with volumes must commit");

	assert_eq!(
		stored_volumes(&setup.database, id).await,
		["/data", "/var/lib/postgresql"]
	);
}

/// Update is clear-then-reinsert: removing a path drops its row, re-adding it
/// brings the row back.
#[tokio::test]
async fn upsert_replaces_the_volume_set() {
	let setup = setup().await;
	let id = Uuid::new_v4();
	let mut conn = setup.database.acquire().await.unwrap();

	for (paths, expected) in [
		(&["/data", "/cache"][..], &["/cache", "/data"][..]),
		(&["/data"], &["/data"]),
		(&[], &[]),
		(&["/data"], &["/data"]),
	] {
		db_helpers::upsert_deployment_in_database(
			&mut conn,
			WithId::new(id, test_deployment()),
			running_details_with_volumes(paths),
		)
		.await
		.unwrap();
		assert_eq!(stored_volumes(&setup.database, id).await, expected);
	}
}

/// Actually executes `m005_deployment_volumes`. A fresh database marks every
/// migration applied without running it, so this rebuilds the pre-m005
/// tables by hand, un-marks the migration, and runs the migration runner.
#[tokio::test]
async fn m005_replaces_the_volume_tables() {
	let setup = setup().await;
	let mut conn = setup.database.acquire().await.unwrap();

	sqlx::query("DROP TABLE deployment_volume")
		.execute(&mut *conn)
		.await
		.unwrap();
	sqlx::query(
		r#"
		CREATE TABLE deployment_volume(
			id UUID NOT NULL PRIMARY KEY,
			name TEXT NOT NULL UNIQUE,
			volume_size INT NOT NULL CHECK(volume_size > 0),
			deleted DATETIME
		);
		"#,
	)
	.execute(&mut *conn)
	.await
	.unwrap();
	sqlx::query(
		r#"
		CREATE TABLE deployment_volume_mount(
			deployment_id UUID NOT NULL,
			volume_id UUID NOT NULL,
			volume_mount_path TEXT NOT NULL,

			PRIMARY KEY(deployment_id, volume_id),
			FOREIGN KEY(deployment_id) REFERENCES deployment(id),
			FOREIGN KEY(volume_id) REFERENCES deployment_volume(id)
		);
		"#,
	)
	.execute(&mut *conn)
	.await
	.unwrap();
	sqlx::query("DELETE FROM migrations WHERE name = 'm005_deployment_volumes'")
		.execute(&mut *conn)
		.await
		.unwrap();

	common::migrations::run_migrations(&mut conn, &common::utils::constants::DATABASE_VERSION)
		.await
		.expect("m005 must apply");

	let tables = sqlx::query(
		"SELECT name FROM sqlite_schema WHERE type = 'table' AND name LIKE 'deployment_volume%'",
	)
	.fetch_all(&mut *conn)
	.await
	.unwrap()
	.into_iter()
	.map(|row| row.get::<String, _>("name"))
	.collect::<Vec<_>>();
	assert_eq!(tables, ["deployment_volume"]);

	let columns = sqlx::query("SELECT name FROM pragma_table_info('deployment_volume')")
		.fetch_all(&mut *conn)
		.await
		.unwrap()
		.into_iter()
		.map(|row| row.get::<String, _>("name"))
		.collect::<Vec<_>>();
	assert_eq!(columns, ["deployment_id", "path"]);

	// And the new table is usable through the same path the WebSocket takes.
	let id = Uuid::new_v4();
	db_helpers::upsert_deployment_in_database(
		&mut conn,
		WithId::new(id, test_deployment()),
		running_details_with_volumes(&["/data"]),
	)
	.await
	.unwrap();
	assert_eq!(stored_volumes(&setup.database, id).await, ["/data"]);
}

/// Deleting the deployment must take its volume rows with it, or the FK
/// blocks the `deployment` delete.
#[tokio::test]
async fn delete_removes_volume_rows() {
	let setup = setup().await;
	let id = Uuid::new_v4();
	let mut conn = setup.database.acquire().await.unwrap();

	db_helpers::upsert_deployment_in_database(
		&mut conn,
		WithId::new(id, test_deployment()),
		running_details_with_volumes(&["/data"]),
	)
	.await
	.unwrap();

	db_helpers::delete_deployment_in_database(&mut conn, id)
		.await
		.expect("delete must succeed with volume rows present");

	assert!(stored_volumes(&setup.database, id).await.is_empty());
}
