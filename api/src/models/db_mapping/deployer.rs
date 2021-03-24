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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Event {
	pub id: String,
	pub time_stamp: String,
	pub action: String,
	pub target: Target,
	pub request: Request,
	pub actor: Actor,
	pub source: Source,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
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
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Request {
	pub id: String,
	pub addr: String,
	pub host: String,
	pub method: String,
	pub useragent: String,
}
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Actor {
	pub name: String,
}
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Source {
	pub addr: String,
	pub instance_id: String,
}
