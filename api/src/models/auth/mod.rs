mod access_token_data;
mod email_validate;
mod oauth;
mod user_api_token_data;

use std::{collections::BTreeMap, net::IpAddr};

use api_models::{models::workspace::WorkspacePermission, utils::Uuid};
use redis::aio::MultiplexedConnection as RedisConnection;

pub use self::{
	access_token_data::*,
	email_validate::*,
	oauth::*,
	user_api_token_data::*,
};
use super::rbac::{self, GOD_USER_ID};
use crate::{utils::Error, Database};

#[derive(Clone, Debug)]
pub enum UserAuthenticationData {
	AccessToken(AccessTokenData),
	ApiToken(ApiTokenData),
}

impl UserAuthenticationData {
	pub async fn parse(
		connection: &mut <Database as sqlx::Database>::Connection,
		redis_connection: &mut RedisConnection,
		jwt_secret_key: &str,
		token: &str,
		accessing_ip: &IpAddr,
	) -> Result<Self, Error> {
		if token.starts_with("patrv1") {
			let api_token = ApiTokenData::decode(
				connection,
				redis_connection,
				token,
				accessing_ip,
			)
			.await?;

			Ok(Self::ApiToken(api_token))
		} else {
			let access_token = AccessTokenData::decode(
				connection,
				redis_connection,
				token,
				jwt_secret_key,
			)
			.await?;

			Ok(Self::AccessToken(access_token))
		}
	}

	pub fn is_api_token(&self) -> bool {
		matches!(self, UserAuthenticationData::ApiToken(_))
	}

	pub fn login_id(&self) -> &Uuid {
		match &self {
			UserAuthenticationData::AccessToken(access_token_data) => {
				&access_token_data.login_id
			}
			UserAuthenticationData::ApiToken(user_api_token_data) => {
				&user_api_token_data.token_id
			}
		}
	}

	pub fn user_id(&self) -> &Uuid {
		match &self {
			UserAuthenticationData::AccessToken(access_token_data) => {
				&access_token_data.user.id
			}
			UserAuthenticationData::ApiToken(user_api_token_data) => {
				&user_api_token_data.user_id
			}
		}
	}

	pub fn workspace_permissions(
		&self,
	) -> &BTreeMap<Uuid, WorkspacePermission> {
		match &self {
			UserAuthenticationData::AccessToken(access) => &access.permissions,
			UserAuthenticationData::ApiToken(api) => &api.permissions,
		}
	}

	pub fn has_access_for_requested_action(
		&self,
		workspace_id: &Uuid,
		resource_id: &Uuid,
		resource_type_id: &Uuid,
		permission_required: &str,
	) -> bool {
		let god_user_id = GOD_USER_ID.get().unwrap();
		if god_user_id == self.user_id() {
			// for god user allow all operations on all workspace
			return true;
		}

		let workspace_permissions = self.workspace_permissions();

		let workspace_permission =
			if let Some(permission) = workspace_permissions.get(workspace_id) {
				permission
			} else {
				return false;
			};

		let allowed = {
			// Check if super user is permitted
			workspace_permission.is_super_admin
		} || {
			// Check if the resource type is allowed
			if let Some(permissions) = workspace_permission
				.resource_type_permissions
				.get(resource_type_id)
			{
				permissions.contains(
					rbac::PERMISSIONS
						.get()
						.unwrap()
						.get(&(*permission_required).to_string())
						.unwrap(),
				)
			} else {
				false
			}
		} || {
			// Check if that specific resource is allowed
			if let Some(permissions) =
				workspace_permission.resource_permissions.get(resource_id)
			{
				permissions.contains(
					rbac::PERMISSIONS
						.get()
						.unwrap()
						.get(&(*permission_required).to_string())
						.unwrap(),
				)
			} else {
				false
			}
		};

		allowed
	}
}
