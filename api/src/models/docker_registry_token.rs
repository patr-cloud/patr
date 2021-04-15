use jsonwebtoken::{errors::Error, Algorithm, EncodingKey, Header};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::utils::settings::Settings;

#[derive(Serialize, Deserialize, Clone, Debug)]
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

#[derive(Serialize, Deserialize, Clone, Debug)]
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
			jti: thread_rng().sample_iter(Alphanumeric).take(32).collect(),
			access,
		}
	}

	pub fn to_string(
		&self,
		private_key: &[u8],
		public_key: &[u8],
	) -> Result<String, Error> {
		let hash: Vec<u8> = Sha256::digest(public_key)
			.to_vec()
			.into_iter()
			.take(30)
			.collect();
		let encoded =
			base32::encode(base32::Alphabet::RFC4648 { padding: false }, &hash);
		let mut kid = String::with_capacity(59);
		for i in 0..encoded.len() {
			let character = encoded.chars().nth(i).unwrap();
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
}
