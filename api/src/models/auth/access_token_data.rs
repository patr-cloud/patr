use std::collections::HashMap;

use api_models::utils::Uuid;
use chrono::{DateTime, Utc};
use eve_rs::AsError;
use jsonwebtoken::{
	Algorithm,
	DecodingKey,
	EncodingKey,
	TokenData,
	Validation,
};
use redis::aio::MultiplexedConnection as RedisConnection;
use serde::{Deserialize, Serialize};

use crate::{
	error,
	models::rbac::WorkspacePermissions,
	redis::is_access_token_revoked,
	utils::Error,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AccessTokenData {
	pub iss: String,
	pub aud: String,
	#[serde(with = "datetime_as_seconds")]
	pub iat: DateTime<Utc>,
	pub typ: String,
	#[serde(with = "datetime_as_seconds")]
	pub exp: DateTime<Utc>,
	pub workspace_permissions: HashMap<Uuid, WorkspacePermissions>,
	pub login_id: Uuid,
	pub user: ExposedUserData,
	// Do we need to add more?
}

impl AccessTokenData {
	pub async fn parse(
		token: String,
		key: &str,
		redis_conn: &mut RedisConnection,
	) -> Result<AccessTokenData, Error> {
		let decode_key = DecodingKey::from_secret(key.as_ref());
		let TokenData { header: _, claims } =
			jsonwebtoken::decode(&token, &decode_key, &{
				let mut validation = Validation::new(Algorithm::HS256);
				validation.validate_exp = false;
				validation
			})?;

		Self::validate_access_token(redis_conn, &claims).await?;

		Ok(claims)
	}

	async fn validate_access_token(
		redis_conn: &mut RedisConnection,
		access_token: &AccessTokenData,
	) -> Result<(), Error> {
		// check whether access token has expired
		if access_token.exp < Utc::now() {
			return Error::as_result()
				.status(401)
				.body(error!(EXPIRED).to_string())?;
		}

		// check whether access token has revoked
		match is_access_token_revoked(redis_conn, access_token).await {
			Ok(false) => (), // access token not revoked hence valid
			_ => {
				// either access token revoked or redis connection error
				return Error::as_result()
					.status(401)
					.body(error!(EXPIRED).to_string());
			}
		}

		Ok(())
	}

	pub fn to_string(&self, key: &str) -> Result<String, Error> {
		let result = jsonwebtoken::encode(
			&Default::default(),
			&self,
			&EncodingKey::from_secret(key.as_ref()),
		)?;

		Ok(result)
	}

	pub fn new(
		iat: DateTime<Utc>,
		exp: DateTime<Utc>,
		workspaces: HashMap<Uuid, WorkspacePermissions>,
		login_id: Uuid,
		user: ExposedUserData,
	) -> Self {
		AccessTokenData {
			iss: String::from("https://api.patr.cloud"),
			aud: String::from("https://*.patr.cloud"),
			iat,
			typ: String::from("accessToken"),
			exp,
			workspace_permissions: workspaces,
			login_id,
			user,
		}
	}
}

// Data about the user that can be exposed in the access token
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExposedUserData {
	pub id: Uuid,
	pub username: String,
	pub first_name: String,
	pub last_name: String,
	pub created: DateTime<Utc>,
}

mod datetime_as_seconds {
	use chrono::{DateTime, TimeZone, Utc};
	use serde::{Deserialize, Deserializer, Serializer};

	pub fn serialize<S>(
		value: &DateTime<Utc>,
		serializer: S,
	) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_i64(value.timestamp())
	}

	pub fn deserialize<'de, D>(
		deserializer: D,
	) -> Result<DateTime<Utc>, D::Error>
	where
		D: Deserializer<'de>,
	{
		i64::deserialize(deserializer)
			.map(|timestamp| Utc.timestamp(timestamp, 0))
	}
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct IpAddressInfo {
	pub country: String,
	pub region: String,
	pub city: String,
	pub loc: String,
	pub timezone: String,
}
