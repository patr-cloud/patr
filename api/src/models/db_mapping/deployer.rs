use crate::utils::get_current_time;
use digest::Digest;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;

pub struct DockerRepository {
	pub id: Vec<u8>,
	pub organisation_id: Vec<u8>,
	pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Event {
	pub id: String,
	pub time_stamp: Duration,
	pub action: String,
	pub target: Target,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Target {
	pub media_type: String,
	pub size: i64,
	pub digest: String,
	pub length: u64,
	pub urls: Vec<String>,
	pub repository: String,
	pub url: String,
	pub tag: String,
	pub request: Request,
	pub actor: Actor,
	pub source: Source,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Request {
	pub id: String,
	pub addr: String,
	pub host: String,
	pub method: String,
	pub user_agent: String,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Actor {}
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Source {
	pub addr: String,
	pub instance_id: String,
}
