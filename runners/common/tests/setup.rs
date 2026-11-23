use std::{collections::BTreeMap, sync::Arc};

use common::{
	actors::{
		db_helpers,
		resource_supervisor::{
			ResourceSupervisor,
			ResourceSupervisorArgs,
			ResourceSupervisorMessage,
		},
	},
	db,
	prelude::*,
};
use models::api::workspace::deployment::*;
use ractor::{Actor, ActorRef, concurrency::JoinHandle};

use crate::mock_executor::*;

/// Isolated test environment with a fresh SQLite DB, mock executor and a
/// running ResourceSupervisor (no upstream WebSocket). Each call to `setup()`
/// creates a completely independent instance — tests can run in parallel
/// safely.
pub struct TestSetup {
	pub database: sqlx::Pool<DatabaseType>,
	#[allow(dead_code)]
	pub config: RunnerSettings<()>,
	pub mock_state: Arc<MockExecutorState>,
	pub supervisor_ref: ActorRef<ResourceSupervisorMessage>,
	_root_handle: JoinHandle<()>,
	_temp_dir: tempfile::TempDir,
}

/// Creates a fresh test environment:
/// 1. Temp directory with a new SQLite file
/// 2. Schema initialized from scratch (all tables, machine types seeded)
/// 3. ResourceSupervisor spawned with MockExecutor
/// 4. Returns TestSetup with refs to everything
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

	let supervisor_name = format!("test-supervisor-{}", Uuid::new_v4());
	let (root_ref, root_handle) = ResourceSupervisor::<MockExecutor>::spawn(
		Some(supervisor_name),
		ResourceSupervisor::new(),
		ResourceSupervisorArgs {
			config: config.clone(),
			database: database.clone(),
			runner_state: mock_state.clone(),
			websocket_ref: None,
		},
	)
	.await
	.expect("failed to spawn ResourceSupervisor");

	// Give the actor tree a moment to start and run initial Reconcile.
	tokio::time::sleep(std::time::Duration::from_millis(100)).await;

	TestSetup {
		database,
		config,
		mock_state,
		supervisor_ref: root_ref,
		_root_handle: root_handle,
		_temp_dir: temp_dir,
	}
}

impl TestSetup {
	/// Send an UpsertResource message to the supervisor for the given
	/// deployment.
	pub fn notify_upsert(&self, deployment_id: Uuid) {
		let _ = self
			.supervisor_ref
			.send_message(ResourceSupervisorMessage::UpsertResource {
				resource_id: deployment_id,
				resource_type: models::rbac::ResourceType::Deployment,
			});
	}

	/// Send a DeleteResource message to the supervisor for the given
	/// deployment.
	pub fn notify_delete(&self, deployment_id: Uuid) {
		let _ = self
			.supervisor_ref
			.send_message(ResourceSupervisorMessage::DeleteResource {
				resource_id: deployment_id,
				resource_type: models::rbac::ResourceType::Deployment,
			});
	}

	/// Insert a minimal test deployment into SQLite. Returns the deployment ID.
	pub async fn create_test_deployment(&self) -> Uuid {
		self.create_test_deployment_with_status(DeploymentStatus::Deploying)
			.await
	}

	/// Insert a test deployment with a specific initial status.
	pub async fn create_test_deployment_with_status(&self, status: DeploymentStatus) -> Uuid {
		let id = Uuid::new_v4();

		let deployment = Deployment {
			name: format!("test-deployment-{}", &id.to_string()[..8]),
			registry: DeploymentRegistry::ExternalRegistry {
				registry: "docker.io".to_string(),
				image_name: "nginx".to_string(),
			},
			image_tag: "latest".to_string(),
			status,
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
