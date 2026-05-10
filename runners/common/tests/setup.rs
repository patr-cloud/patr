use std::{collections::BTreeMap, net::SocketAddr, sync::Arc};

use axum_test::{TestResponse, TestServer};
use common::{
	actors::{
		db_helpers,
		runner_supervisor::{RunnerSupervisor, RunnerSupervisorArgs, RunnerSupervisorMessage},
	},
	db,
	prelude::*,
	routes,
};
use models::{
	ApiRequest,
	api::{ApiEndpoint, workspace::deployment::*},
	utils::Headers,
};
use ractor::{Actor, ActorRef, concurrency::JoinHandle};
use serde::Serialize;

use crate::mock_executor::*;

/// Isolated test environment with a fresh SQLite DB, mock executor,
/// running actor tree, and an HTTP TestServer. Each call to `setup()`
/// creates a completely independent instance — tests can run in parallel
/// safely.
pub struct TestSetup {
	pub database: sqlx::Pool<DatabaseType>,
	#[allow(dead_code)]
	pub config: RunnerSettings<()>,
	pub mock_state: Arc<MockExecutorState>,
	pub supervisor_ref: ActorRef<RunnerSupervisorMessage>,
	pub http: TestServer,
	_root_handle: JoinHandle<()>,
	_temp_dir: tempfile::TempDir,
}

/// Creates a fresh test environment:
/// 1. Temp directory with a new SQLite file
/// 2. Schema initialized from scratch (all tables, machine types seeded)
/// 3. RunnerSupervisor actor tree spawned with MockExecutor
/// 4. Returns TestSetup with refs to everything
pub async fn setup() -> TestSetup {
	let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
	let db_path = temp_dir.path().join("test.db");

	let mock_state = MockExecutorState::new();

	let config = RunnerSettings {
		mode: RunnerMode::SelfHosted {
			password_pepper: "test-pepper-at-least-32-chars-long!!".to_string(),
			// Must match the hardcoded secret in authenticator.rs:133.
			jwt_secret: "keyboard cat".to_string(),
		},
		environment: RunningEnvironment::Development,
		database: DatabaseConfig {
			file: db_path.to_string_lossy().to_string(),
			connection_limit: 5,
		},
		bind_address: SocketAddr::from(([127, 0, 0, 1], 0)),
		data: (),
	};

	let database = db::connect(&config.database)
		.await
		.expect("failed to connect to test database");
	db::initialize(&database)
		.await
		.expect("failed to initialize test database");

	let supervisor_name = format!("test-supervisor-{}", Uuid::new_v4());
	let (root_ref, root_handle) = RunnerSupervisor::<MockExecutor>::spawn(
		Some(supervisor_name),
		RunnerSupervisor::new(),
		RunnerSupervisorArgs {
			config: config.clone(),
			database: database.clone(),
			runner_state: mock_state.clone(),
		},
	)
	.await
	.expect("failed to spawn RunnerSupervisor");

	// Give the actor tree a moment to start and run initial Reconcile.
	tokio::time::sleep(std::time::Duration::from_millis(100)).await;

	let state = AppState::<MockExecutor> {
		database: database.clone(),
		config: config.clone(),
		runner_state: mock_state.clone(),
		supervisor_ref: root_ref.clone(),
	};
	let http = TestServer::new(routes::setup_routes(&state).await);

	TestSetup {
		database,
		config,
		mock_state,
		supervisor_ref: root_ref,
		http,
		_root_handle: root_handle,
		_temp_dir: temp_dir,
	}
}

impl TestSetup {
	/// Make a typed API call using `ApiRequest<E>`. Mirrors
	/// `api/tests/setup.rs::TestSetup::make_api_call`.
	pub async fn make_api_call<E>(&self, request: ApiRequest<E>) -> TestResponse
	where
		E: ApiEndpoint,
		E::RequestBody: Serialize,
		E::RequestHeaders: Headers,
		E::RequestPath: std::fmt::Display,
		E::RequestQuery: Serialize,
	{
		let path = request.path.to_string();
		let query = serde_qs::to_string(&request.query).unwrap_or_default();
		let full_path = if query.is_empty() {
			path
		} else {
			format!("{path}?{query}")
		};

		let mut req = self.http.method(E::METHOD, &full_path);
		for (name, value) in request.headers.to_header_map().iter() {
			req = req.add_header(name.clone(), value.to_str().unwrap());
		}
		req.json(&request.body).await
	}

	/// Send an UpsertResource message to the supervisor for the given
	/// deployment.
	pub fn notify_upsert(&self, deployment_id: Uuid) {
		let _ = self
			.supervisor_ref
			.send_message(RunnerSupervisorMessage::UpsertResource {
				resource_id: deployment_id,
				resource_type: models::rbac::ResourceType::Deployment,
			});
	}

	/// Send a DeleteResource message to the supervisor for the given
	/// deployment.
	pub fn notify_delete(&self, deployment_id: Uuid) {
		let _ = self
			.supervisor_ref
			.send_message(RunnerSupervisorMessage::DeleteResource {
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
		};

		let mut conn = self
			.database
			.acquire()
			.await
			.expect("failed to acquire connection");
		db_helpers::create_deployment_in_database(
			&mut conn,
			WithId::new(id, deployment),
			running_details,
		)
		.await
		.expect("failed to insert test deployment");

		id
	}
}
