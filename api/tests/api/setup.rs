use std::net::SocketAddr;

use api::{
	app::AppState,
	prelude::ClientType,
	routes::api_patr_cloud as routes,
	utils::config::{
		AppConfig,
		CloudflareConfig,
		DatabaseConfig,
		IpInfoConfig,
		LogsConfig,
		MetricsConfig,
		OpenTelemetryConfig,
		RedisConfig,
		RunningEnvironment,
		S3Config,
		TracingConfig,
	},
};
use axum_test::TestServer;
use rand::{Rng, distributions::Alphanumeric};
use testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner as _};
use testcontainers_modules::{minio::MinIO, postgres::Postgres, redis::Redis};
use tokio::net::TcpListener;

pub struct TestSetup {
	pub server: TestServer,
	pub state: AppState,
	pub s3_container: ContainerAsync<MinIO>,
	pub postgres_container: ContainerAsync<Postgres>,
	pub redis_container: ContainerAsync<Redis>,
}

/// Helps setup the test server and database for API tests. This is used by all
/// API tests, so it should be kept up to date and working.
pub async fn setup() -> Result<TestSetup, anyhow::Error> {
	let bind_address = TcpListener::bind("127.0.0.1:0").await?.local_addr()?;

	let password_pepper = rand::thread_rng()
		.sample_iter(Alphanumeric)
		.map(char::from)
		.take(32)
		.collect::<String>();

	let jwt_secret = rand::thread_rng()
		.sample_iter(Alphanumeric)
		.map(char::from)
		.take(64)
		.collect::<String>();

	let s3_container = MinIO::default().start().await?;

	let s3 = S3Config {
		endpoint: format!(
			"http://{}:{}",
			s3_container.get_host().await?.to_string(),
			s3_container.get_host_port_ipv4(9000).await?
		),
		region: "us-east-1".to_string(),
		bucket: "test-bucket".to_string(),
		key: "minioadmin".to_string(),
		secret: "minioadmin".to_string(),
	};

	let postgres_container = Postgres::default()
		.with_db_name("api")
		.with_user("user")
		.with_password("password")
		.with_name("postgis/postgis")
		.with_tag("13-master")
		.start()
		.await?;

	let database = DatabaseConfig {
		host: postgres_container.get_host().await?.to_string(),
		port: postgres_container.get_host_port_ipv4(5432).await?,
		user: "user".to_string(),
		password: "password".to_string(),
		database: "api".to_string(),
		connection_limit: 10,
	};

	let redis_container = Redis::default().with_tag("7").start().await?;

	let redis = RedisConfig {
		host: redis_container.get_host().await?.to_string(),
		port: redis_container.get_host_port_ipv4(6379).await?,
		user: None,
		password: None,
		database: 0,
		secure: false,
	};

	let config = AppConfig {
		bind_address,
		api_base_path: String::from("/"),
		password_pepper,
		jwt_secret,
		primary_hosted_domain: String::from("testonpatr.cloud"),
		environment: if cfg!(debug_assertions) {
			RunningEnvironment::Development
		} else {
			RunningEnvironment::Production
		},
		s3,
		database,
		redis,
		cloudflare: CloudflareConfig {
			api_key: "fake-api-key".to_string(),
			account_id: "fake-account-id".to_string(),
			worker_namespace_id: "fake-worker-namespace-id".to_string(),
			turnstile_secret: "1x0000000000000000000000000000000AA".to_string(),
			primary_hosted_zone_id: "fake-hosted-zone-id".to_string(),
			ingress_script_name: "ingress".to_string(),
		},
		opentelemetry: OpenTelemetryConfig {
			tracing: TracingConfig {
				endpoint: "".to_string(),
			},
			logs: LogsConfig {
				endpoint: "".to_string(),
			},
			metrics: MetricsConfig {
				endpoint: "".to_string(),
				username: "".to_string(),
				password: "".to_string(),
			},
		},
		ipinfo: IpInfoConfig {
			token: "ipinfo-token".to_string(),
		},
	};

	_ = api::utils::setup_tracing(&config);

	let state = api::build_state(config).await;

	api::db::initialize(&state)
		.await
		.map_err(|e| anyhow::anyhow!("error initializing database: {e}"))?;

	let server = TestServer::builder()
		.http_transport_with_ip_port(Some(bind_address.ip()), Some(bind_address.port()))
		.save_cookies()
		.build(
			routes::setup_routes(&state, ClientType::WebDashboard)
				.await
				.into_make_service_with_connect_info::<SocketAddr>(),
		)?;

	Ok(TestSetup {
		server,
		state,
		s3_container,
		postgres_container,
		redis_container,
	})
}
