use eve_rs::AsError;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, Transaction};

use crate::{db, error, utils::Error};
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
	pub domain_id: Vec<u8>,
	pub sub_domain: String,
	pub path: String,
	pub persistence: bool,
	pub datacenter: String,
}

impl Deployment {
	async fn get_full_image(
		mut self,
		connection: &mut Transaction<'_, MySql>,
	) -> Result<Self, Error> {
		if self.registry == "registry.docker.vicara.co" {
			match &self.repository_id {
				Some(repo_id) => {
					let docker_repository =
						db::get_docker_repository_by_id(connection, &repo_id)
							.await?
							.status(404)
							.body(error!(REPOSITORY_NOT_FOUND).to_string())?;

					self.image_name = Some(docker_repository.name);
				}
				None => Error::as_result()
					.status(500)
					.body(error!(SERVER_ERROR).to_string())?,
			}
		}

		Ok(Deployment {
			id: self.id,
			name: self.name,
			registry: self.registry,
			repository_id: self.repository_id,
			image_name: self.image_name,
			image_tag: self.image_tag,
			domain_id: self.domain_id,
			sub_domain: self.sub_domain,
			path: self.path,
			persistence: self.persistence,
			datacenter: self.datacenter,
		})
	}
}

pub struct DeploymentConfig {
	pub id: Vec<u8>,
	pub name: String,
	pub registry: String,
	pub image_name: String,
	pub image_tag: String,
	pub domain_id: Vec<u8>,
	pub sub_domain: String,
	pub path: String,
	pub port_list: Vec<u8>,
	pub env_variable_list: Vec<EnvVariable>,
	pub volume_mount_list: Vec<VolumeMount>,
}

pub struct MachineType {
	pub id: Vec<u8>,
	pub name: String,
	pub cpu_count: u8,
	pub memory_count: f32,
	pub gpu_type_id: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct EnvVariable {
	pub deployment_id: Vec<u8>,
	pub name: String,
	pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct VolumeMount {
	pub deployment_id: Vec<u8>,
	pub name: String,
	pub path: String,
}
