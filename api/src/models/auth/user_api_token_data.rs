use std::collections::HashMap;

use api_models::utils::Uuid;
use chrono::{DateTime, Utc};
use eve_rs::AsError;
use sqlx::types::ipnetwork::IpNetwork;

use crate::{
	db,
	error,
	models::rbac::WorkspacePermissions,
	utils::Error,
	Database,
};

// Token Structure: patrv1.{token}.loginId
#[derive(Clone, Debug)]
pub struct UserApiTokenData {
	pub token_id: Uuid,
	pub user_id: Uuid,
	pub token_nbf: Option<DateTime<Utc>>,
	pub token_exp: Option<DateTime<Utc>>,
	pub allowed_ips: Option<Vec<IpNetwork>>,
	pub created: DateTime<Utc>,
	pub revoked: Option<DateTime<Utc>>,
	pub workspace_permissions: HashMap<Uuid, WorkspacePermissions>,
}

impl UserApiTokenData {
	pub async fn parse(
		connection: &mut <Database as sqlx::Database>::Connection,
		token: &str,
	) -> Result<Self, Error> {
		let mut chunked_token = token.splitn(3, '.');

		let (plain_token, login_id) = match (
			chunked_token.next(),
			chunked_token.next(),
			chunked_token.next(),
		) {
			(Some(version), Some(plain_token), Some(login_id))
				if version == "patrv1" =>
			{
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

		let token_details =
			db::get_currently_active_api_token_by_id(connection, &login_id)
				.await?
				.status(401)
				.body(error!(UNAUTHORIZED).to_string())?;

		// todo:
		// hash the raw token and check whether
		// it is equivalent to original one
		let hashed_user_provided_token = plain_token;

		// todo: validate whether the ip is in allowed ip address list

		if hashed_user_provided_token != token_details.token_hash {
			log::info!("Hashed user provided token doesn't match with token_hash in db");
			return Err(Error::empty()
				.status(401)
				.body(error!(UNAUTHORIZED).to_string()));
		}

		// token matches, so now return permissions for token
		let workspace_permissions =
			db::get_all_permissions_for_api_token(connection, &login_id)
				.await?;

		Ok(Self {
			token_id: token_details.token_id,
			user_id: token_details.user_id,
			token_nbf: token_details.token_nbf,
			token_exp: token_details.token_exp,
			allowed_ips: token_details.allowed_ips,
			created: token_details.created,
			revoked: token_details.revoked,
			workspace_permissions,
		})
	}
}
