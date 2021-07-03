use std::{fmt::Display, str::FromStr};

use eve_rs::AsError;
use serde::{Deserialize, Serialize};
use sqlx::types::ipnetwork::IpNetwork;

use crate::{db, error, utils::Error, Database};

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
	pub deployed_image: Option<String>,
}

impl Deployment {
	pub async fn get_full_image(
		&self,
		connection: &mut <Database as sqlx::Database>::Connection,
	) -> Result<String, Error> {
		if self.registry == "registry.docker.vicara.co" {
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
				"registry.docker.vicara.co",
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

#[derive(sqlx::Type, Debug)]
#[sqlx(type_name = "DEPLOYMENT_RUNNER_STATUS", rename_all = "lowercase")]
pub enum DeploymentStatus {
	Alive,
	Starting,
	#[sqlx(rename = "shutting down")]
	ShuttingDown,
	Dead,
}

impl Display for DeploymentStatus {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Alive => write!(f, "alive"),
			Self::Starting => write!(f, "starting"),
			Self::ShuttingDown => write!(f, "shutting down"),
			Self::Dead => write!(f, "dead"),
		}
	}
}

impl FromStr for DeploymentStatus {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"alive" => Ok(Self::Alive),
			"starting" => Ok(Self::Starting),
			"shutting down" => Ok(Self::ShuttingDown),
			"dead" => Ok(Self::Dead),
			_ => Error::as_result()
				.status(500)
				.body(error!(WRONG_PARAMETERS).to_string()),
		}
	}
}

pub struct DeploymentRunner {
	pub id: Vec<u8>,
	pub last_updated: u64,
	pub container_id: Option<Vec<u8>>,
}

#[derive(Clone)]
pub struct DeploymentApplicationServer {
	pub server_ip: IpNetwork,
	pub server_type: String,
}

pub struct DeploymentRunnerDeployment {
	pub deployment_id: Vec<u8>,
	pub runner_id: Vec<u8>,
	pub last_updated: u64,
	pub current_server: IpNetwork,
	pub status: DeploymentStatus,
}
