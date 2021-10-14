use std::{fmt::Display, str::FromStr};

use eve_rs::AsError;
use serde::{Deserialize, Serialize};

use crate::{
	db,
	error,
	utils::{settings::Settings, Error},
	Database,
};

pub struct DockerRepository {
	pub id: Vec<u8>,
	pub organisation_id: Vec<u8>,
	pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct EventData {
	pub events: Vec<Event>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Event {
	pub id: String,
	pub timestamp: String,
	pub action: String,
	pub target: Target,
	pub request: Request,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Target {
	pub media_type: String,
	pub size: i64,
	pub digest: String,
	pub length: u64,
	pub repository: String,
	pub url: String,
	pub tag: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Request {
	pub id: String,
	pub addr: String,
	pub host: String,
	pub method: String,
	pub useragent: String,
}

pub struct Deployment {
	pub id: Vec<u8>,
	pub name: String,
	pub registry: String,
	pub repository_id: Option<Vec<u8>>,
	pub image_name: Option<String>,
	pub image_tag: String,
	pub status: DeploymentStatus,
	pub deployed_image: Option<String>,
	pub digitalocean_app_id: Option<String>,
	pub region: String,
	pub domain_name: Option<String>,
	pub horizontal_scale: i16,
	pub machine_type: DeploymentMachineType,
	pub organisation_id: Vec<u8>,
}

impl Deployment {
	pub async fn get_full_image(
		&self,
		connection: &mut <Database as sqlx::Database>::Connection,
		config: &Settings,
	) -> Result<String, Error> {
		if self.registry == "registry.patr.cloud" {
			let docker_repository = db::get_docker_repository_by_id(
				&mut *connection,
				self.repository_id
					.as_ref()
					.status(500)
					.body(error!(SERVER_ERROR).to_string())?,
			)
			.await?
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;

			let organisation = db::get_organisation_info(
				&mut *connection,
				&docker_repository.organisation_id,
			)
			.await?
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;

			Ok(format!(
				"{}/{}/{}",
				config.docker_registry.registry_url,
				organisation.name,
				docker_repository.name
			))
		} else {
			Ok(format!(
				"{}/{}",
				self.registry,
				self.image_name
					.as_ref()
					.status(500)
					.body(error!(SERVER_ERROR).to_string())?
			))
		}
	}
}

#[allow(dead_code)]
pub struct DeploymentRequestLogs {
	id: i64,
	deplyoment_id: Vec<u8>,
	timestamp: u64,
	ip_address: String,
	ip_address_location: (f64, f64),
	method: DeploymentRequestMethod,
	host: String,
	protocol: DeploymentRequestProtocol,
	path: String,
	response_time: f64,
}

#[derive(sqlx::Type, Debug)]
#[sqlx(type_name = "DEPLOYMENT_STATUS", rename_all = "lowercase")]
pub enum DeploymentStatus {
	Created,
	Pushed,
	Deploying,
	Running,
	Stopped,
	Errored,
	Deleted,
}

impl Display for DeploymentStatus {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Created => write!(f, "created"),
			Self::Pushed => write!(f, "pushed"),
			Self::Deploying => write!(f, "deploying"),
			Self::Running => write!(f, "running"),
			Self::Stopped => write!(f, "stopped"),
			Self::Errored => write!(f, "errored"),
			Self::Deleted => write!(f, "deleted"),
		}
	}
}

impl FromStr for DeploymentStatus {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"created" => Ok(Self::Created),
			"pushed" => Ok(Self::Pushed),
			"deploying" => Ok(Self::Deploying),
			"running" => Ok(Self::Running),
			"stopped" => Ok(Self::Stopped),
			"errored" => Ok(Self::Errored),
			"deleted" => Ok(Self::Deleted),
			_ => Error::as_result()
				.status(500)
				.body(error!(WRONG_PARAMETERS).to_string()),
		}
	}
}

#[derive(Serialize, Deserialize, Clone, sqlx::Type, Debug, PartialEq)]
pub enum CloudPlatform {
	Aws,
	DigitalOcean,
}

impl Display for CloudPlatform {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Aws => write!(f, "aws"),
			Self::DigitalOcean => write!(f, "do"),
		}
	}
}

impl FromStr for CloudPlatform {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"aws" | "amazon" | "amazon_web_services" => Ok(Self::Aws),
			"do" | "digitalocean" | "digital_ocean" => Ok(Self::DigitalOcean),
			_ => Error::as_result()
				.status(500)
				.body(error!(WRONG_PARAMETERS).to_string()),
		}
	}
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct CNameRecord {
	pub cname: String,
	pub value: String,
}

#[derive(sqlx::Type, Debug)]
#[sqlx(type_name = "DEPLOYMENT_MACHINE_TYPE", rename_all = "lowercase")]
pub enum DeploymentMachineType {
	Micro,
	Small,
	Medium,
	Large,
}

impl Display for DeploymentMachineType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Micro => write!(f, "micro"),
			Self::Small => write!(f, "small"),
			Self::Medium => write!(f, "medium"),
			Self::Large => write!(f, "large"),
		}
	}
}

impl FromStr for DeploymentMachineType {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"micro" => Ok(Self::Micro),
			"small" => Ok(Self::Small),
			"medium" => Ok(Self::Medium),
			"large" => Ok(Self::Large),
			_ => Error::as_result()
				.status(500)
				.body(error!(WRONG_PARAMETERS).to_string()),
		}
	}
}

#[derive(sqlx::Type, Debug)]
#[sqlx(type_name = "DEPLOYMENT_REQUEST_PROTOCOL", rename_all = "lowercase")]
pub enum DeploymentRequestProtocol {
	Http,
	Https,
}

impl Display for DeploymentRequestProtocol {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Http => write!(f, "http"),
			Self::Https => write!(f, "https"),
		}
	}
}

impl FromStr for DeploymentRequestProtocol {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"http" => Ok(Self::Http),
			"https" => Ok(Self::Https),
			_ => Error::as_result()
				.status(500)
				.body(error!(WRONG_PARAMETERS).to_string()),
		}
	}
}

#[derive(sqlx::Type, Debug)]
#[sqlx(type_name = "DEPLOYMENT_REQUEST_METHOD", rename_all = "lowercase")]
pub enum DeploymentRequestMethod {
	Get,
	Post,
	Put,
	Delete,
	Head,
	Options,
	Connect,
	Patch,
}

impl Display for DeploymentRequestMethod {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Get => write!(f, "get"),
			Self::Post => write!(f, "post"),
			Self::Put => write!(f, "put"),
			Self::Delete => write!(f, "delete"),
			Self::Head => write!(f, "head"),
			Self::Options => write!(f, "options"),
			Self::Connect => write!(f, "connect"),
			Self::Patch => write!(f, "patch"),
		}
	}
}

impl FromStr for DeploymentRequestMethod {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"get" => Ok(Self::Get),
			"post" => Ok(Self::Post),
			"put" => Ok(Self::Put),
			"delete" => Ok(Self::Delete),
			"head" => Ok(Self::Head),
			"options" => Ok(Self::Options),
			"connect" => Ok(Self::Connect),
			"patch" => Ok(Self::Patch),
			_ => Error::as_result()
				.status(500)
				.body(error!(WRONG_PARAMETERS).to_string()),
		}
	}
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IpResponse {
	pub status: String,
	#[serde(default)]
	pub message: String,
	pub lat: f64,
	pub lon: f64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AvgDistance {
	pub region: String,
	pub avg_distance: f64,
}
