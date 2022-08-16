use std::collections::HashMap;

use chrono::{DateTime, Utc};
use jsonwebtoken::{
	errors::Error as JWTError,
	Algorithm,
	DecodingKey,
	EncodingKey,
	Header,
	TokenData,
	Validation,
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::utils::settings::Settings;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryToken {
	pub iss: String,
	pub sub: String,
	pub aud: String,
	pub exp: u64,
	pub nbf: u64,
	pub iat: u64,
	pub jti: String,
	pub access: Vec<RegistryTokenAccess>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryTokenAccess {
	pub r#type: String,
	pub name: String,
	pub actions: Vec<String>,
}

impl RegistryToken {
	pub fn new(
		iss: String,
		iat: u64,
		sub: String,
		config: &Settings,
		access: Vec<RegistryTokenAccess>,
	) -> Self {
		Self {
			iss,
			sub,
			aud: config.docker_registry.service_name.clone(),
			exp: iat + (600), // 5 mins
			nbf: iat,
			iat,
			jti: thread_rng()
				.sample_iter(Alphanumeric)
				.map(char::from)
				.take(32)
				.collect(),
			access,
		}
	}

	pub fn to_string(
		&self,
		private_key: &[u8],
		public_key: &[u8],
	) -> Result<String, JWTError> {
		let hash: Vec<u8> = Sha256::digest(public_key)
			.iter()
			.copied()
			.take(30)
			.collect();
		let encoded =
			base32::encode(base32::Alphabet::RFC4648 { padding: false }, &hash);
		let mut kid = String::with_capacity(59);
		for (i, character) in encoded.chars().enumerate() {
			kid.push(character);
			if i % 4 == 3 && i != (encoded.len() - 1) {
				kid.push(':');
			}
		}
		jsonwebtoken::encode(
			&Header {
				alg: Algorithm::ES256,
				kid: Some(kid),
				..Default::default()
			},
			&self,
			&EncodingKey::from_ec_pem(private_key)?,
		)
	}

	pub fn parse(token: &str, public_key: &[u8]) -> Result<Self, JWTError> {
		let decode_key = DecodingKey::from_ec_pem(public_key)?;
		let TokenData { header: _, claims } =
			jsonwebtoken::decode(token, &decode_key, &{
				let mut validation = Validation::new(Algorithm::ES256);
				validation.validate_exp = false;
				validation
			})?;
		Ok(claims)
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DockerRegistryListImagesResponse {
	pub repositories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DockerRegistryImageListTagsResponse {
	pub name: String,
	pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventData {
	pub events: Vec<Event>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Action {
	Push,
	Pull,
	Delete,
	Mount,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Event {
	pub id: String,
	pub timestamp: DateTime<Utc>,
	pub action: Action,
	pub target: Target,
	pub request: Request,
	pub actor: Actor,
	pub source: Source,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Target {
	#[serde(default, skip_serializing_if = "String::is_empty")]
	pub media_type: String,
	#[serde(default)]
	pub size: u64,
	#[serde(default, skip_serializing_if = "String::is_empty")]
	pub digest: String,
	#[serde(default)]
	pub length: u64,
	pub repository: String,
	#[serde(default, skip_serializing_if = "String::is_empty")]
	pub from_repository: String,
	#[serde(default, skip_serializing_if = "String::is_empty")]
	pub url: String,
	#[serde(default, skip_serializing_if = "String::is_empty")]
	pub tag: String,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub references: Vec<TargetReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TargetReference {
	pub media_type: String,
	pub size: u64,
	pub digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Request {
	pub id: String,
	pub addr: String,
	pub host: String,
	pub method: String,
	pub useragent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Actor {
	#[serde(default, skip_serializing_if = "String::is_empty")]
	pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Source {
	pub addr: String,
	#[serde(
		alias = "instanceID",
		default,
		skip_serializing_if = "String::is_empty"
	)]
	pub instance_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerRepositoryManifest {
	pub history: Vec<V1CompatibilityHolder>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerRepositoryManifestHistory {
	pub history: Vec<V1CompatibilityHolder>,
}

#[derive(Serialize, Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct V1CompatibilityHolder {
	pub v1_compatibility: String,
}

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct V1Compatibility {
	pub container_config: DockerRepositoryExposedPort,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct DockerRepositoryExposedPort {
	pub exposed_ports: Option<HashMap<String, Value>>,
}
