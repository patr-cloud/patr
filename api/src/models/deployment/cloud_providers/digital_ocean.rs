use std::net::{Ipv4Addr, Ipv6Addr};

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

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum DropletStatus {
	New,
	Active,
	Off,
	Archive,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum Type {
	UNSPECIFIED,
	DEFAULT,
	PRIMARY,
	ALIAS,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum Engine {
	UNSET,
	MYSQL,
	PG,
	REDIS,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DigitalOceanImage {
	pub id: u64,
	pub distribution: String,
	pub slug: String,
	pub public: bool,
	pub regions: Vec<String>,
	pub created_at: String,
	pub r#type: String,
	pub min_disk_size: u32,
	pub size_gigabytes: f64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IpV4Address {
	pub ip_address: Ipv4Addr,
	pub netmask: Ipv4Addr,
	pub gateway: String,
	pub r#type: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IpV6Address {
	pub ip_address: Ipv6Addr,
	pub netmask: Ipv6Addr,
	pub gateway: String,
	pub r#type: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NetworkDetails {
	pub v4: Vec<IpV4Address>,
	pub v6: Vec<IpV6Address>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NextBackupWindow {
	pub start: String,
	pub end: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RegionDetails {
	pub name: String,
	pub slug: String,
	pub sizes: Vec<String>,
	pub features: Vec<String>,
	pub available: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SizeObject {
	pub slug: String,
	pub memory: u64,
	pub vcpus: u16,
	pub disk: u64,
	pub transfer: f64,
	pub price_monthly: f64,
	pub price_hourly: f64,
	pub regions: Vec<String>,
	pub available: bool,
	pub description: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DropletResponse {
	pub droplets: Vec<DropletDetails>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DropletDetails {
	pub id: u64,
	pub name: String,
	pub memory: u64,
	pub vcpus: u16,
	pub disk: u16,
	pub locked: bool,
	pub created_at: String,
	pub status: String,
	pub backup_ids: Vec<String>,
	pub snapshot_ids: Vec<String>,
	pub features: Vec<String>,
	pub region: RegionDetails,
	pub image: DigitalOceanImage,
	pub size: SizeObject,
	pub size_slug: String,
	pub networks: NetworkDetails,
	pub next_backup_window: Option<NextBackupWindow>,
	pub tags: Vec<String>,
	pub vpc_uuid: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct DropletRequest {
	pub region: String,
	pub name: String,
	pub size: String,
	pub image: String,
	// stores ssh fingerprint
	// after image, these details are not mandatory
	pub ssh_keys: Option<Vec<String>>,
	pub backups: Option<bool>,
	pub ipv6: Option<bool>,
	pub private_networking: Option<bool>,
	pub vpc_uuid: Option<String>,
	pub user_data: Option<String>,
	pub volumes: Option<Vec<String>>,
	pub tags: Option<Vec<String>>,
}

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
	pub region: Option<String>,
	pub domains: Option<Vec<Domains>>,
	pub services: Option<Vec<Services>>,
	pub static_sites: Option<Vec<StaticSites>>,
	pub jobs: Option<Vec<Jobs>>,
	pub workers: Option<Vec<Workers>>,
	pub databases: Option<Vec<Databases>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Domains {
	pub domain: String,
	// Default unspecified
	pub r#type: Option<Type>,
	pub wildcard: bool,
	pub zone: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Services {
	// ^[a-z][a-z0-9-]{0,30}[a-z0-9]$
	pub name: String,
	pub git: Option<Git>,
	pub github: Option<GitProviders>,
	pub gitlab: Option<GitProviders>,
	pub image: Option<Image>,
	pub dockerfile_path: Option<String>,
	pub build_command: Option<String>,
	pub run_command: Option<String>,
	pub source_dir: Option<String>,
	pub envs: Option<Vec<Envs>>,
	pub environment_slug: Option<String>,
	pub instance_count: Option<u64>,
	pub instance_size_slug: Option<String>,
	pub cors: Option<Cors>,
	pub health_check: Option<HealthCheck>,
	pub http_port: Option<Vec<u64>>,
	pub internal_ports: Option<Vec<u64>>,
	pub routes: Option<Vec<Routes>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StaticSites {
	// Required, ^[a-z][a-z0-9-]{0,30}[a-z0-9]$
	pub name: String,
	pub git: Option<Git>,
	pub github: Option<GitProviders>,
	pub gitlab: Option<GitProviders>,
	pub image: Option<Image>,
	pub dockerfile_path: Option<String>,
	pub build_command: Option<String>,
	pub run_command: Option<String>,
	pub source_dir: Option<String>,
	pub envs: Option<Vec<Envs>>,
	pub environment_slug: Option<String>,
	pub index_document: Option<String>,
	pub error_document: Option<String>,
	pub catchall_document: Option<String>,
	pub output_dir: Option<String>,
	pub cors: Option<Cors>,
	pub routes: Option<Vec<Routes>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Jobs {
	// Required, ^[a-z][a-z0-9-]{0,30}[a-z0-9]$
	pub name: String,
	pub git: Option<Git>,
	pub github: Option<GitProviders>,
	pub gitlab: Option<GitProviders>,
	pub image: Option<Image>,
	pub dockerfile_path: Option<String>,
	pub build_command: Option<String>,
	pub run_command: Option<String>,
	pub source_dir: Option<String>,
	pub envs: Option<Vec<Envs>>,
	pub environment_slug: Option<String>,
	pub instance_count: Option<u64>,
	pub instance_size_slug: Option<String>,
	pub kind: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Workers {
	// Required, ^[a-z][a-z0-9-]{0,30}[a-z0-9]$
	pub name: String,
	pub git: Option<Git>,
	pub github: Option<GitProviders>,
	pub gitlab: Option<GitProviders>,
	pub image: Option<Image>,
	pub dockerfile_path: Option<String>,
	pub build_command: Option<String>,
	pub run_command: Option<String>,
	pub source_dir: Option<String>,
	pub envs: Option<Vec<Envs>>,
	pub environment_slug: Option<String>,
	pub instance_count: Option<u64>,
	pub instance_size_slug: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Databases {
	pub cluster_name: Option<String>,
	// Required, ^[a-z][a-z0-9-]{0,30}[a-z0-9]$
	pub name: String,
	pub db_name: Option<String>,
	pub db_user: Option<String>,
	pub engine: Option<Engine>,
	pub production: Option<bool>,
	pub version: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Git {
	pub branch: Option<String>,
	pub repo_clone_url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GitProviders {
	pub branch: Option<String>,
	pub deploy_on_push: Option<bool>,
	// example: digitalocean/sample-golang
	pub repo: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Image {
	pub registry: Option<String>,
	pub registry_type: Option<String>,
	pub repository: Option<String>,
	pub tag: Option<String>,
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
	pub path: Option<String>,
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
pub struct App {
	pub active_deployment: Option<ActiveDeployment>,
	pub created_at: String,
	pub default_ingress: String,
	pub domains: Option<AppDomains>,
	pub id: String,
	pub in_progress_deployment: Option<DeploymentProgress>,
	// Not sure about removing the option
	pub last_deployment_created_at: Option<String>,
	// Add field pub last_deployment_updated_at: Option<String>
	pub live_domain: String,
	pub live_url: String,
	pub live_url_base: String,
	pub owner_uuid: String,
	pub region: GeographicInformation,
	pub spec: AppSpec,
	pub tier_slug: String,
	pub updated_at: String,
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
	pub static_sites: Option<Vec<StaticSites>>,
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
	pub static_sites: Option<Vec<StaticSites>>,
	pub tier_slug: Option<String>,
	pub updated_at: Option<String>,
	pub workers: Option<Vec<ComponentList>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GeographicInformation {
	pub continent: String,
	pub data_centers: Vec<String>,
	pub default: Option<bool>,
	pub disabled: Option<bool>,
	pub flag: String,
	pub label: String,
	pub reason: Option<String>,
	pub slug: String,
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
