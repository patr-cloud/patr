use std::{collections::BTreeMap, net::SocketAddr, sync::Once};

use api::{
	app::AppState,
	prelude::ClientType,
	routes::{
		api_patr_cloud,
		loki_patr_cloud,
		registry_patr_cloud,
		registry_patr_cloud::{endpoint::RegistryEndpoint, request::RegistryUnprocessedApiRequest},
	},
	utils::config::{
		AppConfig,
		CloudflareConfig,
		DatabaseConfig,
		EmailConfig,
		GitHubOAuthConfig,
		IpInfoConfig,
		LogsConfig,
		MetricsConfig,
		OpenTelemetryConfig,
		RedisConfig,
		RunningEnvironment,
		S3Config,
		SocialLoginConfig,
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
	testcontainers::{
		ContainerAsync,
		GenericImage,
		ImageExt,
		core::{IntoContainerPort, WaitFor},
		runners::AsyncRunner as _,
	},
};
use tokio::net::TcpListener;
use wiremock::{
	Mock,
	MockServer,
	ResponseTemplate,
	matchers::{method, path_regex},
};

static TRACING: Once = Once::new();

#[allow(dead_code)]
pub struct TestSetup {
	api: TestServer,
	registry: TestServer,
	loki: TestServer,
	state: AppState,
	s3_container: ContainerAsync<MinIO>,
	postgres_container: ContainerAsync<Postgres>,
	redis_container: ContainerAsync<Redis>,
	loki_container: ContainerAsync<GenericImage>,
	cloudflare_mock: MockServer,
	permission_ids: BTreeMap<String, Uuid>,
}

impl TestSetup {
	/// The test database pool. Exposed for helpers that need to tweak DB state
	/// directly (e.g. marking a domain as verified) without going through the
	/// real API flow.
	pub(crate) fn database(&self) -> &sqlx::Pool<api::prelude::DatabaseType> {
		&self.state.database
	}

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

	/// Make a raw HTTP call to the loki TestServer.
	pub async fn make_loki_call(
		&self,
		method: http::Method,
		path: &str,
		headers: Vec<(http::HeaderName, &str)>,
		body: Vec<u8>,
	) -> TestResponse {
		let mut req = self.loki.method(method, path);
		for (name, value) in headers {
			req = req.add_header(name, value);
		}
		if body.is_empty() {
			req.await
		} else {
			req.bytes(axum::body::Bytes::from(body)).await
		}
	}

	/// Get the direct URL to the upstream Loki container (for querying logs).
	pub fn upstream_loki_url(&self) -> &str {
		&self.state.config.opentelemetry.logs.endpoint
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

	/// Clear all rate limit keys from Redis. Useful in tests to reset rate
	/// limit state after setup helpers have made API calls.
	pub async fn clear_rate_limits(&self) {
		use rustis::commands::GenericCommands;

		let keys: Vec<String> = self
			.state
			.redis
			.keys("rateLimit:*")
			.await
			.expect("failed to fetch rate limit keys");

		if !keys.is_empty() {
			self.state
				.redis
				.del(keys)
				.await
				.expect("failed to delete rate limit keys");
		}
	}
}

/// Helps setup the test server and database for API tests. This is used by all
/// API tests, so it should be kept up to date and working.
pub async fn setup() -> Result<TestSetup, anyhow::Error> {
	// Bind listeners now and pass them to axum::serve below. Axum-test's
	// `http_transport_with_ip_port` has a drop-then-rebind race that collides
	// with other nextest processes; handing over a live listener avoids it.
	let api_listener = TcpListener::bind("127.0.0.1:0").await?;
	let registry_listener = TcpListener::bind("127.0.0.1:0").await?;
	let loki_listener = TcpListener::bind("127.0.0.1:0").await?;

	let api_bind_address = api_listener.local_addr()?;

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

	let loki_config = r#"
auth_enabled: true
target: all
server:
  http_listen_address: "0.0.0.0"
  http_listen_port: 3100
common:
  path_prefix: /tmp/loki
  replication_factor: 1
memberlist:
  join_members: []
ingester:
  lifecycler:
    address: 127.0.0.1
    ring:
      kvstore:
        store: inmemory
      replication_factor: 1
    final_sleep: 0s
  chunk_idle_period: 1h
  max_chunk_age: 1h
  chunk_target_size: 1048576
  chunk_retain_period: 30s
storage_config:
  tsdb_shipper:
    active_index_directory: /tmp/loki/tsdb_shipper/active_index
    cache_location: /tmp/loki/tsdb_shipper/cache
  filesystem:
    directory: /tmp/loki/chunks
compactor:
  working_directory: /tmp/loki/compactor
schema_config:
  configs:
    - from: 2023-01-01
      store: tsdb
      object_store: filesystem
      schema: v13
      index:
        prefix: index_
        period: 24h
limits_config:
  allow_structured_metadata: true
  ingestion_rate_mb: 64
  ingestion_burst_size_mb: 128
  otlp_config:
    resource_attributes:
      attributes_config:
        - action: index_label
          attributes:
            - runner_id
            - workspace_id
            - deployment_id
            - deployment_name
            - service_name
            - job
"#;

	let loki_container = GenericImage::new("grafana/loki", "3.2.0")
		.with_exposed_port(3100.tcp())
		.with_wait_for(WaitFor::message_on_stderr("Loki started"))
		.with_copy_to(
			"/etc/loki/test-config.yaml",
			loki_config.as_bytes().to_vec(),
		)
		.with_cmd(["-config.file=/etc/loki/test-config.yaml"])
		.start()
		.await?;

	let redis_container = Redis::default().with_tag("7").start().await?;

	let redis = RedisConfig {
		host: redis_container.get_host().await?.to_string(),
		port: redis_container.get_host_port_ipv4(6379).await?,
		user: None,
		password: None,
		database: 0,
		secure: false,
	};

	let cloudflare_mock = MockServer::start().await;
	mount_cloudflare_mocks(&cloudflare_mock).await;

	let email = EmailConfig {
		host: "smtp.sendgrid.net".to_string(),
		port: 587,
		from: "no-reply@patr.cloud".to_string(),
		username: "apikey".to_string(),
		password: "SG.fake-api-key".to_string(),
		secure: true,
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
		email,
		s3: s3.clone(),
		database,
		redis,
		cloudflare: CloudflareConfig {
			api_key: "fake-api-key".to_string(),
			account_id: "fake-account-id".to_string(),
			worker_namespace_id: "fake-worker-namespace-id".to_string(),
			turnstile_secret: "1x0000000000000000000000000000000AA".to_string(),
			primary_hosted_zone_id: "fake-hosted-zone-id".to_string(),
			base_url: format!("{}/client/v4/", cloudflare_mock.uri()),
		},
		opentelemetry: OpenTelemetryConfig {
			tracing: TracingConfig {
				endpoint: "".to_string(),
			},
			logs: LogsConfig {
				endpoint: format!(
					"http://{}:{}",
					loki_container.get_host().await?,
					loki_container.get_host_port_ipv4(3100).await?
				),
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
		social_login: SocialLoginConfig {
			github: GitHubOAuthConfig {
				client_id: "fake-github-client-id".to_string(),
				client_secret: "fake-github-client-secret".to_string(),
				callback_url: "http://localhost:3000/login/github".to_string(),
				connect_callback_url: "http://localhost:3000/profile/github/callback".to_string(),
			},
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

	api::worker::initialize(&state)
		.await
		.map_err(|e| anyhow::anyhow!("error initializing worker: {e}"))?;

	// Seed deployment machine types (the table is created by db::initialize
	// but never populated).
	sqlx::query(
		r#"
		INSERT INTO deployment_machine_type (id, cpu_count, memory_count)
		VALUES
			('d47b2c5a-0001-4000-8000-000000000001', 1, 1),
			('d47b2c5a-0002-4000-8000-000000000002', 2, 4),
			('d47b2c5a-0004-4000-8000-000000000004', 4, 8)
		ON CONFLICT DO NOTHING;
		"#,
	)
	.execute(&state.database)
	.await
	.map_err(|e| anyhow::anyhow!("error seeding machine types: {e}"))?;

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

	let api = TestServer::builder().save_cookies().build(axum::serve(
		api_listener,
		api_patr_cloud::setup_routes(&state, ClientType::WebDashboard)
			.await
			.into_make_service_with_connect_info::<SocketAddr>(),
	));

	let registry = TestServer::builder().build(axum::serve(
		registry_listener,
		registry_patr_cloud::setup_routes(&state)
			.await
			.into_make_service_with_connect_info::<SocketAddr>(),
	));

	let loki = TestServer::builder().build(axum::serve(
		loki_listener,
		loki_patr_cloud::setup_routes(&state)
			.await
			.into_make_service_with_connect_info::<SocketAddr>(),
	));

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
		loki,
		state,
		s3_container,
		postgres_container,
		redis_container,
		loki_container,
		cloudflare_mock,
		permission_ids,
	})
}

/// Helper to build a Cloudflare API envelope response.
fn cf_success(result: serde_json::Value) -> ResponseTemplate {
	ResponseTemplate::new(200).set_body_json(serde_json::json!({
		"success": true,
		"errors": [],
		"messages": [],
		"result": result
	}))
}

/// Mount wiremock stubs for all Cloudflare API endpoints used by the
/// application. Each stub returns a minimal valid response in the
/// standard Cloudflare envelope format.
async fn mount_cloudflare_mocks(server: &MockServer) {
	// POST /zones/*/custom_hostnames — AddCustomHostname
	Mock::given(method("POST"))
		.and(path_regex(r"^/client/v4/zones/[^/]+/custom_hostnames$"))
		.respond_with(cf_success(serde_json::json!({
			"id": "mock-custom-hostname-id",
			"hostname": "example.com",
			"ssl": {
				"status": "pending_validation",
				"method": "txt",
				"type": "dv",
				"validation_records": []
			},
			"status": "pending"
		})))
		.mount(server)
		.await;

	// PATCH /zones/*/custom_hostnames/* — EditCustomHostname
	Mock::given(method("PATCH"))
		.and(path_regex(
			r"^/client/v4/zones/[^/]+/custom_hostnames/[^/]+$",
		))
		.respond_with(cf_success(serde_json::json!({
			"id": "mock-custom-hostname-id",
			"hostname": "example.com",
			"ssl": {
				"status": "active",
				"method": "txt",
				"type": "dv",
				"validation_records": []
			},
			"status": "active"
		})))
		.mount(server)
		.await;

	// GET /zones/*/custom_hostnames/* — GetCustomHostnameDetails
	Mock::given(method("GET"))
		.and(path_regex(
			r"^/client/v4/zones/[^/]+/custom_hostnames/[^/]+$",
		))
		.respond_with(cf_success(serde_json::json!({
			"id": "mock-custom-hostname-id",
			"hostname": "example.com",
			"ssl": {
				"status": "pending_validation",
				"method": "txt",
				"type": "dv",
				"validation_records": [
					{
						"txt_name": "_acme-challenge.example.com",
						"txt_value": "mock-txt-value"
					}
				]
			},
			"status": "pending",
			"ownership_verification": {
				"type": "txt",
				"name": "_cf-custom-hostname.example.com",
				"value": "mock-ownership-value"
			}
		})))
		.mount(server)
		.await;

	// DELETE /zones/*/custom_hostnames/* — DeleteCustomHostname
	Mock::given(method("DELETE"))
		.and(path_regex(
			r"^/client/v4/zones/[^/]+/custom_hostnames/[^/]+$",
		))
		.respond_with(cf_success(serde_json::json!({
			"id": "mock-custom-hostname-id"
		})))
		.mount(server)
		.await;

	// GET /zones — ListZones
	Mock::given(method("GET"))
		.and(path_regex(r"^/client/v4/zones$"))
		.respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
			"success": true,
			"errors": [],
			"messages": [],
			"result": [{
				"id": "mock-zone-id",
				"name": "testonpatr.cloud",
				"status": "active",
				"paused": false,
				"type": "full",
				"development_mode": 0,
				"name_servers": ["ns1.mock.com", "ns2.mock.com"]
			}],
			"result_info": {
				"page": 1,
				"per_page": 20,
				"total_pages": 1,
				"count": 1,
				"total_count": 1
			}
		})))
		.mount(server)
		.await;

	// POST /zones — CreateZone
	Mock::given(method("POST"))
		.and(path_regex(r"^/client/v4/zones$"))
		.respond_with(cf_success(serde_json::json!({
			"id": "mock-zone-id",
			"name": "testonpatr.cloud",
			"status": "active",
			"paused": false,
			"type": "full",
			"development_mode": 0,
			"name_servers": ["ns1.mock.com", "ns2.mock.com"]
		})))
		.mount(server)
		.await;

	// DELETE /zones/* — DeleteZone
	Mock::given(method("DELETE"))
		.and(path_regex(r"^/client/v4/zones/[^/]+$"))
		.respond_with(cf_success(serde_json::json!({
			"id": "mock-zone-id"
		})))
		.mount(server)
		.await;

	// PUT /accounts/*/storage/kv/namespaces/*/values/* — WriteKey
	Mock::given(method("PUT"))
		.and(path_regex(
			r"^/client/v4/accounts/[^/]+/storage/kv/namespaces/[^/]+/values/.+$",
		))
		.respond_with(cf_success(serde_json::json!(null)))
		.mount(server)
		.await;

	// DELETE /accounts/*/storage/kv/namespaces/*/values/* — DeleteKey
	Mock::given(method("DELETE"))
		.and(path_regex(
			r"^/client/v4/accounts/[^/]+/storage/kv/namespaces/[^/]+/values/.+$",
		))
		.respond_with(cf_success(serde_json::json!(null)))
		.mount(server)
		.await;

	// POST /accounts/*/cfd_tunnel — CreateTunnel
	Mock::given(method("POST"))
		.and(path_regex(r"^/client/v4/accounts/[^/]+/cfd_tunnel$"))
		.respond_with(cf_success(serde_json::json!({
			"id": "00000000-0000-0000-0000-000000000000",
			"name": "mock-tunnel",
			"created_at": "2024-01-01T00:00:00Z",
			"deleted_at": null,
			"connections": [],
			"metadata": {}
		})))
		.mount(server)
		.await;

	// GET /accounts/*/cfd_tunnel/*/token — GetTunnelToken
	// (must be before the more general GET tunnel pattern)
	Mock::given(method("GET"))
		.and(path_regex(
			r"^/client/v4/accounts/[^/]+/cfd_tunnel/[^/]+/token$",
		))
		.respond_with(cf_success(serde_json::json!("mock-tunnel-token-value")))
		.mount(server)
		.await;

	// GET /accounts/*/cfd_tunnel/* — GetTunnel
	Mock::given(method("GET"))
		.and(path_regex(r"^/client/v4/accounts/[^/]+/cfd_tunnel/[^/]+$"))
		.respond_with(cf_success(serde_json::json!({
			"id": "00000000-0000-0000-0000-000000000000",
			"name": "mock-tunnel",
			"created_at": "2024-01-01T00:00:00Z",
			"deleted_at": null,
			"connections": [],
			"metadata": {}
		})))
		.mount(server)
		.await;

	// PUT /accounts/*/cfd_tunnel/*/configurations — UpdateTunnelConfig
	Mock::given(method("PUT"))
		.and(path_regex(
			r"^/client/v4/accounts/[^/]+/cfd_tunnel/[^/]+/configurations$",
		))
		.respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
			"success": true,
			"errors": [],
			"messages": [],
			"result": {}
		})))
		.mount(server)
		.await;

	// DNS records — catch-all for GET/POST/PUT/DELETE
	Mock::given(path_regex(r"^/client/v4/zones/[^/]+/dns_records"))
		.respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
			"success": true,
			"errors": [],
			"messages": [],
			"result": [],
			"result_info": {
				"page": 1,
				"per_page": 20,
				"total_pages": 1,
				"count": 0,
				"total_count": 0
			}
		})))
		.mount(server)
		.await;
}
