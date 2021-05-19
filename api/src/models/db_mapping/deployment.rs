use serde::{Deserialize, Serialize};

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
	pub image_name: String,
	pub image_tag: String,
	pub domain_id: Vec<u8>,
	pub sub_domain: String,
	pub path: String,
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
