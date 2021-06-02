use std::collections::HashMap;

use eve_rs::AsError;
use serde::{Deserialize, Serialize};

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
	pub upgrade_path_id: Vec<u8>,
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

pub struct VolumeMount {
	pub deployment_id: Vec<u8>,
	pub name: String,
	pub path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct MachineType {
	pub cpu_count: u8,
	pub memory_count: f32,
}

pub struct DeploymentUpgradePath {
	pub id: Vec<u8>,
	pub name: String,
	pub default_machine_type: Vec<u8>,
}

pub enum DeploymentEntryPointValue {
	Deployment {
		deployment_id: Vec<u8>,
		deployment_port: u16,
	},
	Redirect {
		url: String,
	},
	Proxy {
		url: String,
	},
}

#[derive(sqlx::Type, Debug)]
#[sqlx(type_name = "DEPLOYMENT_ENTRY_POINT_TYPE", rename_all = "lowercase")]
pub enum DeploymentEntryPointType {
	Deployment,
	Redirect,
	Proxy,
}

pub struct DeploymentEntryPoint {
	pub id: Vec<u8>,
	pub sub_domain: Option<String>,
	pub domain_id: Vec<u8>,
	pub path: String,
	pub entry_point_type: DeploymentEntryPointValue,
}

pub enum DeploymentRepositoryImage {
	// image comes from the internal registry
	Internal {
		repository_id: Vec<u8>,
		image_name: String,
	},
	External {
		registry: String,
		image_name: String,
	},
}

pub struct DeploymentConfiguration {
	pub id: Vec<u8>,
	pub name: String,
	pub image: DeploymentRepositoryImage,
	pub image_tag: String,
	pub ports: Vec<u16>,
	pub volumes: Vec<VolumeMount>,
	pub environment_variables: HashMap<String, String>,
	pub upgrade_path: DeploymentUpgradePath,
	pub upgrade_path_machines: Vec<MachineType>,
}

impl DeploymentConfiguration {
	pub async fn get_full_image_name(&self) -> String {
		match &self.image {
			DeploymentRepositoryImage::Internal {
				repository_id: _,
				image_name,
			} => {
				format!("registry.docker.vicara.co/{}", image_name)
			}
			DeploymentRepositoryImage::External {
				registry,
				image_name,
			} => {
				format!("{}/{}", registry, image_name)
			}
		}
	}
}
