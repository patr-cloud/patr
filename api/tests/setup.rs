use std::{collections::BTreeMap, net::SocketAddr, sync::Once};

use api::{
	app::AppState,
	prelude::ClientType,
	routes::{
		api_patr_cloud,
		registry_patr_cloud,
		registry_patr_cloud::{endpoint::RegistryEndpoint, request::RegistryUnprocessedApiRequest},
	},
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
use aws_credential_types::Credentials;
use aws_sdk_s3::Client as S3Client;
use axum_test::{TestResponse, TestServer};
use http::header;
use models::{
	ApiRequest,
	api::ApiEndpoint,
	rbac::Permission,
	utils::{Headers, Uuid},
};
use preprocess::Preprocessable;
use rand::{RngExt as _, distr::Alphanumeric};
use serde::Serialize;
use testcontainers_modules::{
	minio::MinIO,
	postgres::Postgres,
	redis::Redis,
	testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner as _},
};
use tokio::net::TcpListener;

static TRACING: Once = Once::new();

#[allow(dead_code)]
pub struct TestSetup {
	api: TestServer,
	registry: TestServer,
	state: AppState,
	s3_container: ContainerAsync<MinIO>,
	postgres_container: ContainerAsync<Postgres>,
	redis_container: ContainerAsync<Redis>,
	permission_ids: BTreeMap<String, Uuid>,
}

impl TestSetup {
	/// Make a typed API call using `ApiRequest<E>`.
	///
	/// All headers (including `authorization` and `user_agent`) are provided
	/// through the typed headers struct in the request.
	pub async fn make_api_call<E>(&self, request: ApiRequest<E>) -> TestResponse
	where
		E: ApiEndpoint,
		E::RequestBody: Serialize,
		E::RequestHeaders: Headers,
		E::RequestPath: std::fmt::Display,
		E::RequestQuery: Serialize,
	{
		let path_str = request.path.to_string();
		let query_str = serde_qs::to_string(&request.query).unwrap_or_default();
		let full_path = if query_str.is_empty() {
			path_str
		} else {
			format!("{}?{}", path_str, query_str)
		};

		let mut req = self.api.method(E::METHOD, &full_path);
		let header_map = request.headers.to_header_map();
		for (name, value) in header_map.iter() {
			req = req.add_header(name.clone(), value.to_str().unwrap());
		}
		req.json(&request.body).await
	}

	/// Make a typed registry call using `RegistryUnprocessedApiRequest<E>`.
	///
	/// All endpoint-specific headers (including `authorization`) are provided
	/// through the typed headers struct. `User-Agent` is added automatically
	/// since registry endpoint headers don't include it.
	pub async fn make_registry_call<E>(
		&self,
		request: RegistryUnprocessedApiRequest<E>,
	) -> TestResponse
	where
		E: RegistryEndpoint,
		E::RequestPath: std::fmt::Display,
		E::RequestHeaders: Headers,
		E::RequestQuery: Serialize,
		<E::RequestPath as Preprocessable>::Processed: Send,
		<E::RequestQuery as Preprocessable>::Processed: Send,
	{
		let path_str = request.path.to_string();
		let query_str = serde_qs::to_string(&request.query).unwrap_or_default();
		let full_path = if query_str.is_empty() {
			path_str
		} else {
			format!("{}?{}", path_str, query_str)
		};

		let mut req = self.registry.method(E::METHOD, &full_path);
		req = req.add_header(header::USER_AGENT, "cargo-test");
		let header_map = request.headers.to_header_map();
		for (name, value) in header_map.iter() {
			req = req.add_header(name.clone(), value.to_str().unwrap());
		}

		let body_bytes = axum::body::to_bytes(request.body, usize::MAX)
			.await
			.unwrap();
		if body_bytes.is_empty() {
			req.await
		} else {
			req.bytes(body_bytes).await
		}
	}

	/// Look up the UUID of a permission by the strongly-typed `Permission`
	/// enum. Uses the cached `permission_ids` populated at setup time.
	pub fn get_permission_id(&self, permission: Permission) -> Uuid {
		let key = permission.to_string();
		*self
			.permission_ids
			.get(&key)
			.unwrap_or_else(|| panic!("permission '{}' not found in cached IDs", key))
	}
}

/// Helps setup the test server and database for API tests. This is used by all
/// API tests, so it should be kept up to date and working.
pub async fn setup() -> Result<TestSetup, anyhow::Error> {
	let api_bind_address = TcpListener::bind("127.0.0.1:0").await?.local_addr()?;
	let registry_bind_address = TcpListener::bind("127.0.0.1:0").await?.local_addr()?;

	let password_pepper = rand::rng()
		.sample_iter(Alphanumeric)
		.map(char::from)
		.take(32)
		.collect::<String>();

	let jwt_secret = rand::rng()
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
		force_path_style: true,
	};

	let postgres_container = Postgres::default()
		.with_db_name("api")
		.with_user("user")
		.with_password("password")
		.with_name("postgis/postgis")
		.with_tag("18-3.6")
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
		bind_address: api_bind_address,
		api_base_path: String::from("/"),
		password_pepper,
		jwt_secret,
		primary_hosted_domain: String::from("testonpatr.cloud"),
		environment: if cfg!(debug_assertions) {
			RunningEnvironment::Development
		} else {
			RunningEnvironment::Production
		},
		s3: s3.clone(),
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
			token: "".to_string(),
		},
	};

	TRACING.call_once(|| {
		use tracing_subscriber::{
			filter::LevelFilter,
			fmt::{Layer as FmtLayer, format::FmtSpan},
			prelude::*,
		};

		tracing_subscriber::registry()
			.with(
				FmtLayer::new()
					.with_test_writer()
					.with_span_events(FmtSpan::NONE)
					.event_format(
						tracing_subscriber::fmt::format()
							.pretty()
							.with_ansi(true)
							.with_file(false)
							.without_time()
							.with_target(false)
							.with_source_location(false),
					)
					.with_filter(
						tracing_subscriber::filter::Targets::new()
							.with_target("api", LevelFilter::TRACE)
							.with_target("models", LevelFilter::TRACE),
					)
					.with_filter(LevelFilter::TRACE),
			)
			.init();
	});

	let state = api::build_state(config).await;

	api::db::initialize(&state)
		.await
		.map_err(|e| anyhow::anyhow!("error initializing database: {e}"))?;

	// Create S3 bucket for registry blob/manifest storage.
	// Must use force_path_style(true) for MinIO in testcontainers.
	let s3_client = S3Client::from_conf(
		aws_sdk_s3::Config::builder()
			.behavior_version_latest()
			.region(aws_sdk_s3::config::Region::new(s3.region.clone()))
			.endpoint_url(s3.endpoint.clone())
			.credentials_provider(
				Credentials::builder()
					.access_key_id(&s3.key)
					.secret_access_key(&s3.secret)
					.provider_name("Static")
					.build(),
			)
			.force_path_style(true)
			.build(),
	);
	s3_client
		.create_bucket()
		.bucket(&s3.bucket)
		.send()
		.await
		.map_err(|e| anyhow::anyhow!("error creating S3 bucket: {e}"))?;

	let api = TestServer::builder()
		.http_transport_with_ip_port(Some(api_bind_address.ip()), Some(api_bind_address.port()))
		.save_cookies()
		.build(
			api_patr_cloud::setup_routes(&state, ClientType::WebDashboard)
				.await
				.into_make_service_with_connect_info::<SocketAddr>(),
		);

	let registry = TestServer::builder()
		.http_transport_with_ip_port(
			Some(registry_bind_address.ip()),
			Some(registry_bind_address.port()),
		)
		.build(
			registry_patr_cloud::setup_routes(&state)
				.await
				.into_make_service_with_connect_info::<SocketAddr>(),
		);

	let permission_ids: BTreeMap<String, Uuid> = {
		use sqlx::Row;
		let rows = sqlx::query("SELECT id, name FROM permission")
			.fetch_all(&state.database)
			.await
			.map_err(|e| anyhow::anyhow!("error fetching permissions: {e}"))?;
		rows.into_iter()
			.map(|row| {
				let id: Uuid = row.get("id");
				let name: String = row.get("name");
				(name, id)
			})
			.collect()
	};

	Ok(TestSetup {
		api,
		registry,
		state,
		s3_container,
		postgres_container,
		redis_container,
		permission_ids,
	})
}
