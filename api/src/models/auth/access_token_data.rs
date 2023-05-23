use std::collections::BTreeMap;

use ::redis::aio::MultiplexedConnection as RedisConnection;
use api_models::{models::workspace::WorkspacePermission, utils::Uuid};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{
	Algorithm,
	DecodingKey,
	EncodingKey,
	TokenData,
	Validation,
};
use serde::{Deserialize, Serialize};

use crate::{db, error, redis, service, utils::Error, Database};

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
	pub login_id: Uuid,
	pub user: ExposedUserData,
	#[serde(skip)]
	pub permissions: BTreeMap<Uuid, WorkspacePermission>,
}

impl AccessTokenData {
	pub async fn decode(
		connection: &mut <Database as sqlx::Database>::Connection,
		redis_connection: &mut RedisConnection,
		token: &str,
		key: &str,
	) -> Result<AccessTokenData, Error> {
		let decode_key = DecodingKey::from_secret(key.as_ref());
		let TokenData {
			header: _,
			mut claims,
		} = jsonwebtoken::decode::<Self>(token, &decode_key, &{
			let mut validation = Validation::new(Algorithm::HS256);
			validation.validate_exp = false;
			validation
		})?;

		if !claims
			.is_valid(connection, redis_connection)
			.await
			.unwrap_or(false)
		{
			return Err(Error::empty()
				.status(401)
				.body(error!(EXPIRED).to_string()));
		}

		Ok(claims)
	}

	async fn is_valid(
		&mut self,
		connection: &mut <Database as sqlx::Database>::Connection,
		redis_conn: &mut RedisConnection,
	) -> Result<bool, Error> {
		// check whether access token has expired
		if self.exp < Utc::now() {
			return Ok(false);
		}

		// check whether access token has been revoked
		// check user revocation
		let revoked_timestamp = redis::get_token_revoked_timestamp_for_user(
			redis_conn,
			&self.user.id,
		)
		.await?;
		if matches!(revoked_timestamp, Some(revoked_timestamp) if self.iat < revoked_timestamp)
		{
			return Ok(false);
		}

		// check login revocation
		let revoked_timestamp = redis::get_token_revoked_timestamp_for_login(
			redis_conn,
			&self.login_id,
		)
		.await?;
		if matches!(revoked_timestamp, Some(revoked_timestamp) if self.iat < revoked_timestamp)
		{
			return Ok(false);
		}

		// get permissions from redis
		let permissions = redis::get_user_access_token_permissions(
			redis_conn,
			&self.login_id,
		)
		.await?
		.and_then(|permission| {
			serde_json::from_str::<BTreeMap<Uuid, WorkspacePermission>>(
				&permission,
			)
			.ok()
		});

		self.permissions = if let Some(permissions) = permissions {
			permissions
		} else {
			// If not present in the redis fetch from db
			let all_workspace_permissions =
				db::get_all_workspace_role_permissions_for_user(
					connection,
					&self.user.id,
				)
				.await?;

			// add into redis
			let access_token_ttl =
				service::get_access_token_expiry() + Duration::seconds(60);
			redis::set_user_access_token_permissions(
				redis_conn,
				&self.login_id,
				&serde_json::to_string(&all_workspace_permissions)?,
				Some(&access_token_ttl),
			)
			.await?;

			all_workspace_permissions
		};

		// check workspace revocation
		for workspace_id in self.permissions.keys() {
			let revoked_timestamp =
				redis::get_token_revoked_timestamp_for_workspace(
					redis_conn,
					workspace_id,
				)
				.await?;
			if matches!(revoked_timestamp, Some(revoked_timestamp) if self.iat < revoked_timestamp)
			{
				return Ok(false);
			}
		}

		// check global revocation
		let revoked_timestamp =
			redis::get_global_token_revoked_timestamp(redis_conn).await?;
		if matches!(revoked_timestamp, Some(revoked_timestamp) if self.iat < revoked_timestamp)
		{
			return Ok(false);
		}

		// all checks are passed, hence token has not revoked

		Ok(true)
	}

	pub fn to_string(&self, key: &str) -> Result<String, Error> {
		jsonwebtoken::encode(
			&Default::default(),
			&self,
			&EncodingKey::from_secret(key.as_ref()),
		)
		.map_err(Error::from)
	}

	pub fn new(
		iat: DateTime<Utc>,
		exp: DateTime<Utc>,
		login_id: Uuid,
		user: ExposedUserData,
	) -> Self {
		AccessTokenData {
			iss: String::from("https://api.patr.cloud"),
			aud: String::from("https://*.patr.cloud"),
			iat,
			typ: String::from("accessToken"),
			exp,
			permissions: BTreeMap::new(),
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
			.map(|timestamp| Utc.timestamp_opt(timestamp, 0).unwrap())
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
