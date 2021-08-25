use serde::{Deserialize, Serialize};

// TODO: create a enum or struct for
// region:
// Datacenter Name  Geographic Location             Slug (for the API and doctl)
// NYC1             New York City, United States    nyc1
// NYC2             New York City, United States    nyc2
// NYC3             New York City, United States    nyc3
// AMS2             Amsterdam, the Netherlands      ams2
// AMS3             Amsterdam, the Netherlands      ams3
// SFO1             San Francisco, United States    sfo1
// SFO2             San Francisco, United States    sfo2
// SFO3             San Francisco, United States    sfo3
// SGP1             Singapore                       sgp1
// LON1             London, United Kingdom          lon1
// FRA1             Frankfurt, Germany              fra1
// TOR1             Toronto, Canada                 tor1
// BLR1             Bangalore, India                blr1

#[derive(Debug, Deserialize, Serialize)]
pub struct AppConfig {
	pub spec: AppSpec,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AppSpec {
	// Required
	pub name: String,
	// Enum: "ams" "nyc" "fra"
	// The slug form of the geographical origin of the app.
	// Default: nearest available
	pub region: String,
	pub domains: Vec<Domains>,
	pub services: Vec<Services>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Domains {
	pub domain: String,
	// Default unspecified
	pub r#type: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Services {
	// ^[a-z][a-z0-9-]{0,30}[a-z0-9]$
	pub name: String,
	pub image: Image,
	pub instance_count: u64,
	pub instance_size_slug: String,
	pub http_port: u64,
	pub routes: Vec<Routes>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Image {
	pub registry_type: String,
	pub repository: String,
	pub tag: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Envs {
	// Required, ^[_A-Za-z][_A-Za-z0-9]*$
	pub key: String,
	// Default: "RUN_AND_BUILD_TIME"
	// Enum: "UNSET" "RUN_TIME" "BUILD_TIME" "RUN_AND_BUILD_TIME"
	// RUN_TIME: Made available only at run-time
	// BUILD_TIME: Made available only at build-time
	// RUN_AND_BUILD_TIME: Made available at both build and run-time
	pub scope: Option<String>,
	pub r#type: Option<String>,
	pub value: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Cors {
	pub allow_origins: Option<Vec<AllowOrigins>>,
	pub allow_methods: Option<Vec<String>>,
	pub allow_headers: Option<Vec<String>>,
	pub expose_headers: Option<Vec<String>>,
	pub max_age: Option<String>,
	pub allow_credentials: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HealthCheck {
	pub failure_threshold: Option<u32>,
	pub http_path: Option<String>,
	pub initial_delay_seconds: Option<u32>,
	pub period_seconds: Option<u32>,
	pub success_threshold: Option<u32>,
	pub timeout_seconds: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Routes {
	pub path: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AllowOrigins {
	// [ 1 .. 256 ] characters
	pub exact: Option<String>,
	// [ 1 .. 256 ] characters
	pub prefix: Option<String>,
	// [ 1 .. 256 ] characters
	pub regex: Option<String>,
}

// Reponse body
#[derive(Debug, Deserialize, Serialize)]
pub struct AppHolder {
	pub app: App,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct App {
	pub id: String,
	pub owner_uuid: String,
	pub spec: AppSpec,
	pub last_deployment_active_at: String,
	pub created_at: String,
	pub updated_at: String,
	pub last_deployment_created_at: String,
	pub region: GeographicInformation,
	pub tier_slug: String,
	pub active_deployment: Option<ActiveDeployment>,
	pub default_ingress: Option<String>,
	pub domains: Option<AppDomains>,
	pub in_progress_deployment: Option<DeploymentProgress>,
	// Not sure about removing the option
	// Add field pub last_deployment_updated_at: Option<String>
	pub live_domain: Option<String>,
	pub live_url: Option<String>,
	pub live_url_base: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ActiveDeployment {
	pub cause: String,
	pub cloned_form: Option<String>,
	pub created_at: String,
	pub id: String,
	pub jobs: Option<Vec<ComponentList>>,
	pub phase: String,
	pub phase_last_updated_at: String,
	pub progress: Option<AppsDeploymentProgress>,
	pub services: Option<Vec<ComponentList>>,
	pub spec: AppSpec,
	pub tier_slug: String,
	pub updated_at: String,
	pub workers: Option<Vec<ComponentList>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AppDomains {
	pub id: String,
	pub phase: String,
	pub spec: Domains,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeploymentProgress {
	pub cause: Option<String>,
	pub cloned_from: Option<String>,
	pub created_at: Option<String>,
	pub id: Option<String>,
	pub jobs: Option<Vec<ComponentList>>,
	pub phase: Option<String>,
	pub phase_last_updated_at: Option<String>,
	pub progress: Option<AppsDeploymentProgress>,
	pub services: Option<Vec<ComponentList>>,
	pub spec: Option<AppSpec>,
	pub tier_slug: Option<String>,
	pub updated_at: Option<String>,
	pub workers: Option<Vec<ComponentList>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GeographicInformation {
	pub slug: String,
	pub label: String,
	pub flag: String,
	pub continent: String,
	pub data_centers: Vec<String>,
	pub default: Option<bool>,
	pub disabled: Option<bool>,
	pub reason: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ComponentList {
	pub name: String,
	pub source_commit_hash: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AppsDeploymentProgress {
	pub error_steps: Option<u32>,
	pub pending_steps: Option<u32>,
	pub running_steps: Option<u32>,
	pub success_steps: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Auth {
	pub auths: Registry,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Registry {
	#[serde(rename = "registry.digitalocean.com")]
	pub registry: AuthToken,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthToken {
	pub auth: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RedeployAppRequest {
	pub force_build: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AppDeploymentsResponse {
	pub deployments: Vec<AppDeploymentResponse>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AppDeploymentResponse {
	pub id: String,
	// add more later if required
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AppAggregateLogsResponse {
	pub live_url: String,
	// add more later if required
}
