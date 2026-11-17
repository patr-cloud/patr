use common::{db, prelude::*};

/// A runner config around the database at `file`, for the migrations, which
/// take one.
fn settings(file: String) -> RunnerSettings<()> {
	RunnerSettings {
		workspace_id: Uuid::nil(),
		runner_id: Uuid::nil(),
		api_token: "test-token".parse().unwrap(),
		user_agent: "integration-tests/0.1.0".parse().unwrap(),
		environment: RunningEnvironment::Development,
		database: DatabaseConfig {
			file,
			connection_limit: 5,
		},
		data: (),
	}
}

/// A freshly initialized database in a temp directory.
async fn fresh_database() -> (sqlx::Pool<DatabaseType>, tempfile::TempDir) {
	let temp_dir = tempfile::TempDir::new().unwrap();
	let db_path = temp_dir.path().join("test.db");

	let config = settings(db_path.to_string_lossy().to_string());

	let database = db::connect(&config.database).await.unwrap();
	db::initialize(&database, &config).await.unwrap();

	(database, temp_dir)
}

#[tokio::test]
async fn fresh_db_has_all_tables() {
	let (database, _temp_dir) = fresh_database().await;

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
		"managed_url",
		"secret",
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
	let (database, _temp_dir) = fresh_database().await;

	// Try to record an observed status the runner doesn't know.
	let result = sqlx::query(
		r#"
		INSERT INTO deployment(
			id, name, registry, image_name, image_tag, should_run,
			observed_status, min_horizontal_scale, max_horizontal_scale,
			machine_type, deploy_on_push
		) VALUES (
			'test-id', 'test', 'docker.io', 'nginx', 'latest', 1,
			'stopping', 1, 1, 'b3cf3771fa394281bfdfeb2e65a061b6', 0
		)
		"#,
	)
	.execute(&database)
	.await;

	let err = result.unwrap_err().to_string();
	assert!(
		err.contains("deployment_chk_observed_status_enum"),
		"expected error to mention constraint name, got: {err}"
	);
}

/// The `deployment` table as it was before `m007_local_state_rework`.
const PRE_M007_DEPLOYMENT: &str = r#"
	CREATE TABLE deployment(
		id TEXT NOT NULL PRIMARY KEY,
		name TEXT NOT NULL,
		registry TEXT NOT NULL,
		image_name TEXT,
		repository_id TEXT,
		image_tag TEXT NOT NULL,
		status TEXT NOT NULL,
		min_horizontal_scale INTEGER NOT NULL,
		max_horizontal_scale INTEGER NOT NULL,
		machine_type TEXT NOT NULL,
		deploy_on_push BOOLEAN NOT NULL,
		startup_probe_port INTEGER,
		startup_probe_path TEXT,
		startup_probe_port_type TEXT
			CONSTRAINT deployment_chk_startup_probe_port_type_enum
			CHECK(startup_probe_port_type IN ('http')),
		liveness_probe_port INTEGER,
		liveness_probe_path TEXT,
		liveness_probe_port_type TEXT
			CONSTRAINT deployment_chk_liveness_probe_port_type_enum
			CHECK(liveness_probe_port_type IN ('http')),
		current_live_digest TEXT,
		deleted DATETIME,

		CONSTRAINT deployment_chk_registry_image_name_repository_id_exclusivity CHECK(
			(
				registry = 'registry.patr.cloud' AND
				repository_id IS NOT NULL AND
				image_name IS NULL
			) OR (
				registry != 'registry.patr.cloud' AND
				image_name IS NOT NULL AND
				repository_id IS NULL
			)
		),

		CONSTRAINT deployment_chk_status_enum CHECK(
			status IN (
				'created',
				'pushed',
				'deploying',
				'running',
				'stopped',
				'errored',
				'deleted'
			)
		),

		CONSTRAINT deployment_chk_min_horizontal_scale_range CHECK(
			min_horizontal_scale >= 0 AND
			min_horizontal_scale <= 256 AND
			min_horizontal_scale <= max_horizontal_scale
		),

		CONSTRAINT deployment_chk_max_horizontal_scale_range CHECK(
			max_horizontal_scale >= 0 AND
			max_horizontal_scale <= 256 AND
			max_horizontal_scale >= min_horizontal_scale
		),

		CONSTRAINT deployment_chk_image_name_nonempty
			CHECK(LENGTH(TRIM(image_name)) > 0),
		CONSTRAINT deployment_chk_image_tag_nonempty
			CHECK(LENGTH(TRIM(image_tag)) > 0),

		CONSTRAINT deployment_chk_startup_probe_cohesion CHECK(
			(
				startup_probe_port IS NULL AND
				startup_probe_path IS NULL AND
				startup_probe_port_type IS NULL
			) OR (
				startup_probe_port IS NOT NULL AND
				startup_probe_path IS NOT NULL AND
				startup_probe_port_type IS NOT NULL
			)
		),

		CONSTRAINT deployment_chk_liveness_probe_cohesion CHECK(
			(
				liveness_probe_port IS NULL AND
				liveness_probe_path IS NULL AND
				liveness_probe_port_type IS NULL
			) OR (
				liveness_probe_port IS NOT NULL AND
				liveness_probe_path IS NOT NULL AND
				liveness_probe_port_type IS NOT NULL
			)
		),

		FOREIGN KEY(machine_type) REFERENCES deployment_machine_type(id),
		FOREIGN KEY(id, startup_probe_port, startup_probe_port_type)
			REFERENCES deployment_exposed_port(deployment_id, port, port_type),
		FOREIGN KEY(id, liveness_probe_port, liveness_probe_port_type)
			REFERENCES deployment_exposed_port(deployment_id, port, port_type)
	);
"#;

/// The `CREATE` statement of each named table, with all whitespace removed.
async fn table_sql(
	database: &sqlx::Pool<DatabaseType>,
	tables: &[&str],
) -> Vec<(String, Option<String>)> {
	let mut sql = Vec::new();
	for table in tables {
		let statement =
			sqlx::query("SELECT sql FROM sqlite_schema WHERE type = 'table' AND name = $1")
				.bind(table)
				.fetch_optional(database)
				.await
				.unwrap()
				.map(|row| {
					row.get::<String, _>("sql")
						.chars()
						.filter(|c| !c.is_whitespace())
						.collect::<String>()
				});
		sql.push((table.to_string(), statement));
	}
	sql
}

/// Actually executes `m007_local_state_rework` against a database at the
/// pre-m007 schema holding data in every table it could touch. A fresh database
/// marks every migration applied without running it, so this rebuilds the old
/// `deployment` table by hand, fills it, un-marks the migration, and runs the
/// same initialization an upgraded runner does.
#[tokio::test]
async fn m007_migrates_a_populated_database() {
	let (database, temp_dir) = fresh_database().await;

	for statement in [
		"DROP TABLE deployment",
		PRE_M007_DEPLOYMENT,
		"DELETE FROM migrations WHERE name = 'm007_local_state_rework'",
	] {
		sqlx::query(statement).execute(&database).await.unwrap();
	}

	let running = Uuid::new_v4();
	let stopped = Uuid::new_v4();
	let created = Uuid::new_v4();
	let managed_url = Uuid::new_v4();
	let secret = Uuid::new_v4();
	let repository = Uuid::new_v4();
	let machine_type = Uuid::parse_str("b3cf3771fa394281bfdfeb2e65a061b6").unwrap();

	for (id, status, registry, image_name, repository_id) in [
		(running, "running", "docker.io", Some("nginx"), None),
		(
			stopped,
			"stopped",
			"registry.patr.cloud",
			None,
			Some(repository),
		),
		(created, "created", "docker.io", Some("redis"), None),
	] {
		sqlx::query(
			r#"
			INSERT INTO deployment(
				id, name, registry, image_name, repository_id, image_tag, status,
				min_horizontal_scale, max_horizontal_scale, machine_type,
				deploy_on_push, current_live_digest
			) VALUES (
				$1, $2, $3, $4, $5, 'latest', $6, 1, 2, $7, 1, 'sha256:abc'
			)
			"#,
		)
		.bind(id)
		.bind(format!("deployment-{status}"))
		.bind(registry)
		.bind(image_name)
		.bind(repository_id)
		.bind(status)
		.bind(machine_type)
		.execute(&database)
		.await
		.unwrap();
	}

	// Children for the running deployment, its probe included, which points
	// back at `deployment_exposed_port`.
	sqlx::query(
		"INSERT INTO deployment_exposed_port(deployment_id, port, port_type) VALUES ($1, 80, 'http')",
	)
	.bind(running)
	.execute(&database)
	.await
	.unwrap();
	sqlx::query(
		r#"
		UPDATE deployment SET
			startup_probe_port = 80,
			startup_probe_path = '/health',
			startup_probe_port_type = 'http'
		WHERE id = $1
		"#,
	)
	.bind(running)
	.execute(&database)
	.await
	.unwrap();
	for statement in [
		"INSERT INTO deployment_environment_variable(deployment_id, name, value) VALUES ($1, 'PLAIN', 'value')",
		"INSERT INTO deployment_config_mounts(deployment_id, path, file) VALUES ($1, '/etc/app.conf', x'6b6579')",
		"INSERT INTO deployment_volume(deployment_id, path) VALUES ($1, '/data')",
		"INSERT INTO deployment_deploy_history(deployment_id, image_digest, registry, image_tag, image_name, created) VALUES ($1, 'sha256:abc', 'docker.io', 'latest', 'nginx', '2026-10-01T00:00:00Z')",
	] {
		sqlx::query(statement)
			.bind(running)
			.execute(&database)
			.await
			.unwrap();
	}
	sqlx::query(
		"INSERT INTO deployment_environment_variable(deployment_id, name, secret_id) VALUES ($1, 'SECRET', $2)",
	)
	.bind(running)
	.bind(secret)
	.execute(&database)
	.await
	.unwrap();
	sqlx::query(
		"INSERT INTO managed_url(id, host, path, deployment_id, port) VALUES ($1, 'app.example.com', '/', $2, 80)",
	)
	.bind(managed_url)
	.bind(running)
	.execute(&database)
	.await
	.unwrap();
	sqlx::query("INSERT INTO secret(id, last_updated) VALUES ($1, '2026-10-01T00:00:00Z')")
		.bind(secret)
		.execute(&database)
		.await
		.unwrap();
	sqlx::query(
		r#"
		INSERT INTO meta_data(id, value) VALUES
			('user_id', 'someone@example.com'),
			('password_hash', 'hash'),
			('first_name', 'Some'),
			('last_name', 'One')
		"#,
	)
	.execute(&database)
	.await
	.unwrap();

	db::initialize(
		&database,
		&settings(
			temp_dir
				.path()
				.join("test.db")
				.to_string_lossy()
				.to_string(),
		),
	)
	.await
	.expect("m007 must apply to a populated database");

	// The intent carries over: only a stopped deployment shouldn't run.
	let should_run = sqlx::query("SELECT id, should_run FROM deployment")
		.fetch_all(&database)
		.await
		.unwrap()
		.into_iter()
		.map(|row| (row.get::<Uuid, _>("id"), row.get::<bool, _>("should_run")))
		.collect::<std::collections::BTreeMap<_, _>>();
	assert_eq!(
		should_run,
		[(running, true), (stopped, false), (created, true)].into()
	);

	let columns = sqlx::query("SELECT name FROM pragma_table_info('deployment')")
		.fetch_all(&database)
		.await
		.unwrap()
		.into_iter()
		.map(|row| row.get::<String, _>("name"))
		.collect::<Vec<_>>();
	assert!(columns.iter().any(|column| column == "should_run"));
	assert!(columns.iter().any(|column| column == "observed_status"));
	assert!(!columns.iter().any(|column| column == "status"));

	// Nothing has been observed yet.
	let observed =
		sqlx::query("SELECT COUNT(*) AS count FROM deployment WHERE observed_status IS NOT NULL")
			.fetch_one(&database)
			.await
			.unwrap()
			.get::<i64, _>("count");
	assert_eq!(observed, 0);

	// Everything else on the rows survived, the probe and registry included.
	let row = sqlx::query(
		"SELECT registry, repository_id, max_horizontal_scale, current_live_digest FROM deployment WHERE id = $1",
	)
	.bind(stopped)
	.fetch_one(&database)
	.await
	.unwrap();
	assert_eq!(row.get::<String, _>("registry"), "registry.patr.cloud");
	assert_eq!(row.get::<Uuid, _>("repository_id"), repository);
	assert_eq!(row.get::<i64, _>("max_horizontal_scale"), 2);
	assert_eq!(row.get::<String, _>("current_live_digest"), "sha256:abc");
	let row =
		sqlx::query("SELECT startup_probe_port, startup_probe_path FROM deployment WHERE id = $1")
			.bind(running)
			.fetch_one(&database)
			.await
			.unwrap();
	assert_eq!(row.get::<i64, _>("startup_probe_port"), 80);
	assert_eq!(row.get::<String, _>("startup_probe_path"), "/health");

	for (table, expected) in [
		("deployment_exposed_port", 1),
		("deployment_environment_variable", 2),
		("deployment_config_mounts", 1),
		("deployment_volume", 1),
		("deployment_deploy_history", 1),
		("managed_url", 1),
		("secret", 1),
	] {
		let count = sqlx::query(sqlx::AssertSqlSafe(format!(
			"SELECT COUNT(*) AS count FROM {table}"
		)))
		.fetch_one(&database)
		.await
		.unwrap()
		.get::<i64, _>("count");
		assert_eq!(count, expected, "unexpected row count in {table}");
	}

	let violations = sqlx::query("PRAGMA foreign_key_check")
		.fetch_all(&database)
		.await
		.unwrap();
	assert!(violations.is_empty(), "foreign keys must still hold");

	// The self-hosted keys are gone.
	let keys = sqlx::query("SELECT id FROM meta_data")
		.fetch_all(&database)
		.await
		.unwrap()
		.into_iter()
		.map(|row| row.get::<String, _>("id"))
		.collect::<Vec<_>>();
	for key in ["user_id", "password_hash", "first_name", "last_name"] {
		assert!(
			!keys.iter().any(|existing| existing == key),
			"unexpected key: {key}"
		);
	}
	assert!(keys.iter().any(|key| key == "version_major"));

	// And the migrated schema is exactly the fresh one.
	let (fresh, _fresh_dir) = fresh_database().await;
	let tables = ["deployment"];
	assert_eq!(
		table_sql(&database, &tables).await,
		table_sql(&fresh, &tables).await
	);
}
