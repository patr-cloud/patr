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
			.to_vec()
			.into_iter()
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
		let TokenData { header: _, claims } = jsonwebtoken::decode(
			token,
			&decode_key,
			&Validation {
				validate_exp: false,
				algorithms: vec![Algorithm::ES256],
				..Default::default()
			},
		)?;
		Ok(claims)
	}
}
