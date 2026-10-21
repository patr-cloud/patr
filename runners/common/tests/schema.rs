use std::net::{Ipv4Addr, SocketAddr};

use common::{db, prelude::*};

/// A runner config around the database at `file`, for the migrations, which
/// take one.
fn settings(file: String) -> RunnerSettings<()> {
	RunnerSettings {
		mode: RunnerMode::Managed {
			workspace_id: Uuid::nil(),
			runner_id: Uuid::nil(),
			api_token: "test-token".parse().unwrap(),
			user_agent: "integration-tests/0.1.0".parse().unwrap(),
		},
		environment: RunningEnvironment::Development,
		database: DatabaseConfig {
			file,
			connection_limit: 5,
		},
		bind_address: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
		data: (),
	}
}

#[tokio::test]
async fn fresh_db_has_all_tables() {
	let temp_dir = tempfile::TempDir::new().unwrap();
	let db_path = temp_dir.path().join("test.db");

	let config = settings(db_path.to_string_lossy().to_string());

	let database = db::connect(&config.database).await.unwrap();
	db::initialize(&database, &config).await.unwrap();

	let tables = sqlx::query("SELECT name FROM sqlite_schema WHERE type = 'table'")
		.fetch_all(&database)
		.await
		.unwrap()
		.into_iter()
		.map(|row| row.get("name"))
		.collect::<Vec<String>>();

	let expected = [
		"meta_data",
		"migrations",
		"deployment_machine_type",
		"deployment",
		"deployment_environment_variable",
		"deployment_exposed_port",
		"deployment_config_mounts",
		"deployment_deploy_history",
		"deployment_volume",
	];

	for table in expected {
		assert!(
			tables.contains(&table.to_string()),
			"missing table: {table}"
		);
	}
}

#[tokio::test]
async fn named_constraint_violation_includes_constraint_name() {
	let temp_dir = tempfile::TempDir::new().unwrap();
	let db_path = temp_dir.path().join("test.db");

	let config = settings(db_path.to_string_lossy().to_string());

	let database = db::connect(&config.database).await.unwrap();
	db::initialize(&database, &config).await.unwrap();

	// Try to insert a deployment with an invalid status.
	let result = sqlx::query(
		r#"
		INSERT INTO deployment(
			id, name, registry, image_name, image_tag, status,
			min_horizontal_scale, max_horizontal_scale, machine_type,
			deploy_on_push
		) VALUES (
			'test-id', 'test', 'docker.io', 'nginx', 'latest', 'bogus_status',
			1, 1, 'b3cf3771-fa39-4281-bfdf-eb2e65a061b6', 0
		)
		"#,
	)
	.execute(&database)
	.await;

	let err = result.unwrap_err().to_string();
	assert!(
		err.contains("deployment_chk_status_enum"),
		"expected error to mention constraint name, got: {err}"
	);
}
