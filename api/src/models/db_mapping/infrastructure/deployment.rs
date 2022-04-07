use std::{fmt::Display, str::FromStr};

use api_models::{
	models::workspace::infrastructure::deployment::DeploymentStatus,
	utils::Uuid,
};
use eve_rs::AsError;

use crate::{error, utils::Error};

pub struct DockerRepository {
	pub id: Uuid,
	pub workspace_id: Uuid,
	pub name: String,
}

pub struct Deployment {
	pub id: Uuid,
	pub name: String,
	pub registry: String,
	pub repository_id: Option<Uuid>,
	pub image_name: Option<String>,
	pub image_tag: String,
	pub status: DeploymentStatus,
	pub workspace_id: Uuid,
	pub region: Uuid,
	pub min_horizontal_scale: i16,
	pub max_horizontal_scale: i16,
	pub machine_type: Uuid,
	pub deploy_on_push: bool,
}

pub struct DeploymentMachineType {
	pub id: Uuid,
	pub cpu_count: i16,
	pub memory_count: i32,
}

#[derive(sqlx::Type, Debug, Clone, PartialEq)]
#[sqlx(type_name = "DEPLOYMENT_CLOUD_PROVIDER", rename_all = "lowercase")]
pub enum DeploymentCloudProvider {
	Digitalocean,
}

impl Display for DeploymentCloudProvider {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			DeploymentCloudProvider::Digitalocean => write!(f, "digitalocean"),
		}
	}
}

impl FromStr for DeploymentCloudProvider {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"digitalocean" => Ok(Self::Digitalocean),
			_ => Error::as_result()
				.status(500)
				.body(error!(WRONG_PARAMETERS).to_string()),
		}
	}
}

pub struct DeploymentRegion {
	pub id: Uuid,
	pub name: String,
	pub cloud_provider: Option<DeploymentCloudProvider>,
}

pub struct EnvVariable {
	pub deployment_id: Uuid,
	pub name: String,
	pub value: Option<String>,
	pub secret_id: Option<Uuid>,
}
