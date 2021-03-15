use jsonwebtoken::{
	errors::Error, DecodingKey, EncodingKey, TokenData, Validation,
};
use serde_derive::{Deserialize, Serialize};

use crate::models::access_token_data::ExposedUserData;
use crate::models::rbac::OrgPermissions;

// NOTE:
// 1) status of authorization
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OneTimeToken {
	pub iat: u64,
	pub exp: u64,
	pub unique_id: &[u8],
	pub id: &[u8],
	pub typ: String,
}

impl OneTimeToken {
	pub fn parse(token: String, key: &str) -> Result<OneTimeToken, Error> {
		let decode_key = DecodingKey::from_secret(key.as_ref());
		let TokenData { header: _, claims } = jsonwebtoken::decode(
			&token,
			&decode_key,
			&Validation {
				validate_exp: false,
				..Default::default()
			},
		)?;
		Ok(claims)
	}

	pub fn to_string(&self, key: &str) -> Result<String, Error> {
		jsonwebtoken::encode(
			&Default::default(),
			&self,
			&EncodingKey::from_secret(key.as_ref()),
		)
	}

	pub fn new(
		iat: u64,
		exp: u64,
		unique_id: &'static [u8],
		id: &'static [u8],
	) -> Self {
		OneTimeToken {
			iat,
			typ: String::from("authorizationCode"),
			exp,
			unique_id,
			id,
		}
	}
}
