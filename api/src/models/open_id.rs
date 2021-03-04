use std::collections::HashMap;

use jsonwebtoken::{
	errors::Error, DecodingKey, EncodingKey, TokenData, Validation,
};
use serde_derive::{Deserialize, Serialize};

use crate::models::rbac::OrgPermissions;

// NOTE: 
// 1) need to add scope of access.
// 2) consent of resource owner


#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AccessTokenData {
	pub iss: String,
	pub aud: String,
	pub iat: u64,
	pub typ: String,
	pub exp: u64,
	pub orgs: HashMap<String, OrgPermissions>,
	pub user: ExposedUserData,
	// Do we need to add more?
}

impl AccessTokenData {
	pub fn parse(token: String, key: &str) -> Result<AccessTokenData, Error> {
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
		orgs: HashMap<String, OrgPermissions>,
		user: ExposedUserData,
	) -> Self {
		AccessTokenData {
			iss: String::from("https://api.vicara.co"),
			aud: String::from("https://*.vicara.co"),
			iat,
			typ: String::from("accessToken"),
			exp,
			orgs,
			user,
		}
	}
}
