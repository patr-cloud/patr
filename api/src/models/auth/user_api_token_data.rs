use std::{collections::BTreeMap, net::IpAddr};

use ::redis::aio::MultiplexedConnection as RedisConnection;
use api_models::{models::workspace::WorkspacePermission, utils::Uuid};
use chrono::{DateTime, Duration, Utc};
use eve_rs::AsError;
use serde::{Deserialize, Serialize};
use sqlx::types::ipnetwork::IpNetwork;

use crate::{db, error, redis, service, utils::Error, Database};

// Token Structure: patrv1.{token}.loginId
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiTokenData {
	pub token_id: Uuid,
	pub user_id: Uuid,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub token_nbf: Option<DateTime<Utc>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub token_exp: Option<DateTime<Utc>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub allowed_ips: Option<Vec<IpNetwork>>,
	pub created: DateTime<Utc>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub revoked: Option<DateTime<Utc>>,
	pub permissions: BTreeMap<Uuid, WorkspacePermission>,

	// Internal stuff (for validating the token)
	last_validated: DateTime<Utc>,
	token_hash: String,
}

impl ApiTokenData {
	pub async fn decode(
		connection: &mut <Database as sqlx::Database>::Connection,
		redis_connection: &mut RedisConnection,
		token: &str,
		accessing_ip: &IpAddr,
	) -> Result<Self, Error> {
		let mut chunked_token = token.splitn(3, '.');

		let (plain_token, login_id) = match (
			chunked_token.next(),
			chunked_token.next(),
			chunked_token.next(),
		) {
			(Some("patrv1"), Some(plain_token), Some(login_id)) => {
				(plain_token, login_id)
			}
			_ => {
				log::info!("Invalid api token format");
				return Err(Error::empty()
					.status(401)
					.body(error!(UNAUTHORIZED).to_string()));
			}
		};

		let login_id = Uuid::parse_str(login_id)
			.map_err(|err| {
				log::info!("Invalid login id format in api token");
				err
			})
			.status(401)
			.body(error!(UNAUTHORIZED).to_string())?;

		// check if the token exists on redis
		let redis_token_data = None::<ApiTokenData>;
			// redis::get_user_api_token_data(redis_connection, &login_id)
			// 	.await?
			// 	.and_then(|token| {
			// 		serde_json::from_str::<ApiTokenData>(&token).ok()
			// 	});

		let token_data = if let Some(redis_token_data) = redis_token_data {
			if !service::validate_hash(
				plain_token,
				&redis_token_data.token_hash,
			)? {
				log::info!("Hashed user provided token doesn't match with token_hash in db");
				return Err(Error::empty()
					.status(401)
					.body(error!(UNAUTHORIZED).to_string()));
			}

			let is_valid = redis_token_data
				.is_valid(redis_connection)
				.await
				.unwrap_or(false);

			if !is_valid {
				// Recheck the token permissions to ensure that it is still
				// valid
				let token =
					Self::get_full_token_data(connection, &login_id).await?;

				redis::set_user_api_token_data(
					redis_connection,
					&login_id,
					&serde_json::to_string(&token)?,
					Some(&Duration::hours(8)),
				)
				.await?;

				token
			} else {
				redis_token_data
			}
		} else {
			// Validate the token data to make sure the permissions that it has
			// are still valid as per the user

			let token =
				Self::get_full_token_data(connection, &login_id).await?;

			if !service::validate_hash(plain_token, &token.token_hash)? {
				log::info!("Hashed user provided token doesn't match with token_hash in db");
				return Err(Error::empty()
					.status(401)
					.body(error!(UNAUTHORIZED).to_string()));
			}

			// redis::set_user_api_token_data(
			// 	redis_connection,
			// 	&login_id,
			// 	&serde_json::to_string(&token)?,
			// 	Some(&Duration::hours(8)),
			// )
			// .await?;

			token
		};

		if !token_data.is_access_allowed(accessing_ip) {
			return Err(Error::empty()
				.status(401)
				.body(error!(UNAUTHORIZED).to_string()));
		}

		Ok(token_data)
	}

	async fn get_full_token_data(
		connection: &mut <Database as sqlx::Database>::Connection,
		token_id: &Uuid,
	) -> Result<Self, Error> {
		let token_details =
			db::get_active_user_api_token_by_id(connection, token_id)
				.await?
				.status(401)
				.body(error!(UNAUTHORIZED).to_string())?;

		let permissions = service::get_derived_permissions_for_api_token(
			connection, token_id,
		)
		.await?;

		Ok(Self {
			token_id: token_details.token_id,
			user_id: token_details.user_id,
			token_nbf: token_details.token_nbf,
			token_exp: token_details.token_exp,
			allowed_ips: token_details.allowed_ips,
			created: token_details.created,
			revoked: token_details.revoked,
			permissions,

			last_validated: Utc::now(),
			token_hash: token_details.token_hash,
		})
	}

	async fn is_valid(
		&self,
		redis_conn: &mut RedisConnection,
	) -> Result<bool, Error> {
		// check user revocation
		let revoked_timestamp = redis::get_token_revoked_timestamp_for_user(
			redis_conn,
			&self.user_id,
		)
		.await?;
		if matches!(revoked_timestamp, Some(revoked_timestamp) if self.last_validated < revoked_timestamp)
		{
			return Ok(false);
		}

		// check login revocation
		let revoked_timestamp = redis::get_token_revoked_timestamp_for_login(
			redis_conn,
			&self.token_id,
		)
		.await?;
		if matches!(revoked_timestamp, Some(revoked_timestamp) if self.last_validated < revoked_timestamp)
		{
			return Ok(false);
		}

		// check workspace revocation
		for workspace_id in self.permissions.keys() {
			let revoked_timestamp =
				redis::get_token_revoked_timestamp_for_workspace(
					redis_conn,
					workspace_id,
				)
				.await?;
			if matches!(revoked_timestamp, Some(revoked_timestamp) if self.last_validated < revoked_timestamp)
			{
				return Ok(false);
			}
		}

		// check global revocation
		let revoked_timestamp =
			redis::get_global_token_revoked_timestamp(redis_conn).await?;
		if matches!(revoked_timestamp, Some(revoked_timestamp) if self.last_validated < revoked_timestamp)
		{
			return Ok(false);
		}

		// all checks are passed, hence token has not revoked

		Ok(true)
	}

	fn is_access_allowed(&self, accessing_ip: &IpAddr) -> bool {
		let now = Utc::now();

		if self.token_nbf.map_or(false, |nbf| nbf > now) {
			return false;
		}

		if self.token_exp.map_or(false, |exp| now > exp) {
			return false;
		}

		if let Some(allowed_ips) = &self.allowed_ips {
			if !allowed_ips.is_empty() &&
				!allowed_ips
					.iter()
					.any(|network| network.contains(*accessing_ip))
			{
				return false;
			}
		}

		if self.revoked.map_or(false, |revoked_at| revoked_at < now) {
			return false;
		}

		true
	}
}
