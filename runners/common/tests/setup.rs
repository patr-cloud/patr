use std::{collections::BTreeMap, sync::Arc};

use common::{
	actors::{
		controller::{self, Outcome},
		db_helpers,
	},
	db,
	prelude::*,
};
use models::api::workspace::deployment::*;

use crate::mock_executor::*;

/// A runner's local database and a mock executor, without any actors. Tests
/// drive reconciles directly with [`TestSetup::reconcile`].
pub struct TestSetup {
	pub database: sqlx::Pool<DatabaseType>,
	pub config: RunnerSettings<()>,
	pub executor: MockExecutor,
	pub mock_state: Arc<MockExecutorState>,
	_temp_dir: tempfile::TempDir,
}

/// Create a fresh database in a temp dir and a mock executor over an empty
/// backend.
pub async fn setup() -> TestSetup {
	let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
	let db_path = temp_dir.path().join("test.db");

	let mock_state = MockExecutorState::new();

	let config = RunnerSettings {
		workspace_id: Uuid::new_v4(),
		runner_id: Uuid::new_v4(),
		api_token: "test-token".parse().unwrap(),
		user_agent: "integration-tests/0.1.0".parse().unwrap(),
		environment: RunningEnvironment::Development,
		database: DatabaseConfig {
			file: db_path.to_string_lossy().to_string(),
			connection_limit: 5,
		},
		data: (),
	};

	let database = db::connect(&config.database)
		.await
		.expect("failed to connect to test database");
	db::initialize(&database, &config)
		.await
		.expect("failed to initialize test database");

	TestSetup {
		database,
		config,
		executor: MockExecutor {
			state: mock_state.clone(),
		},
		mock_state,
		_temp_dir: temp_dir,
	}
}

impl TestSetup {
	/// Reconcile one owner against the mock backend.
	pub async fn reconcile(&self, owner: Owner) -> Outcome {
		controller::reconcile(&self.executor, &self.config, &self.database, owner)
			.await
			.expect("failed to reconcile")
	}

	/// Write a deployment to the local database, replacing it if it exists.
	pub async fn upsert_deployment(
		&self,
		id: Uuid,
		deployment: Deployment,
		running_details: DeploymentRunningDetails,
	) {
		let mut conn = self
			.database
			.acquire()
			.await
			.expect("failed to acquire connection");
		db_helpers::upsert_deployment_in_database(
			&mut conn,
			WithId::new(id, deployment),
			running_details,
		)
		.await
		.expect("failed to upsert test deployment");
	}

	/// Delete a deployment from the local database.
	pub async fn delete_deployment(&self, id: Uuid) {
		let mut conn = self
			.database
			.acquire()
			.await
			.expect("failed to acquire connection");
		db_helpers::delete_deployment_in_database(&mut conn, id)
			.await
			.expect("failed to delete test deployment");
	}

	/// Insert a test deployment that should be running.
	pub async fn create_test_deployment(&self) -> Uuid {
		self.create_test_deployment_with_should_run(true).await
	}

	/// Insert a test deployment that should, or shouldn't, be running.
	pub async fn create_test_deployment_with_should_run(&self, should_run: bool) -> Uuid {
		let id = Uuid::new_v4();

		let deployment = Deployment {
			name: format!("test-deployment-{}", &id.to_string()[..8]),
			registry: DeploymentRegistry::ExternalRegistry {
				registry: "docker.io".to_string(),
				image_name: "nginx".to_string(),
			},
			image_tag: "latest".to_string(),
			status: if should_run {
				DeploymentStatus::Observed(ObservedDeploymentStatus::Deploying)
			} else {
				DeploymentStatus::Observed(ObservedDeploymentStatus::Stopped)
			},
			should_run,
			runner: Uuid::nil(),
			current_live_digest: None,
			machine_type: Uuid::parse_str("b3cf3771fa394281bfdfeb2e65a061b6").unwrap(),
		};

		let running_details = DeploymentRunningDetails {
			deploy_on_push: false,
			min_horizontal_scale: 1,
			max_horizontal_scale: 1,
			ports: BTreeMap::new(),
			environment_variables: BTreeMap::new(),
			startup_probe: None,
			liveness_probe: None,
			config_mounts: BTreeMap::new(),
			volumes: BTreeMap::new(),
		};

		let mut conn = self
			.database
			.acquire()
			.await
			.expect("failed to acquire connection");
		db_helpers::upsert_deployment_in_database(
			&mut conn,
			WithId::new(id, deployment),
			running_details,
		)
		.await
		.expect("failed to insert test deployment");

		id
	}
}
