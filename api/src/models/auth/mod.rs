mod access_token_data;
mod user_api_token_data;

use std::collections::HashMap;

use api_models::utils::Uuid;
use redis::aio::MultiplexedConnection as RedisConnection;

pub use self::{access_token_data::*, user_api_token_data::*};
use super::rbac::{self, WorkspacePermissions, GOD_USER_ID};
use crate::{utils::Error, Database};

#[derive(Clone, Debug)]
pub enum UserAuthenticationData {
	AccessToken(AccessTokenData),
	ApiToken(UserApiTokenData),
}

impl UserAuthenticationData {
	pub async fn parse(
		connection: &mut <Database as sqlx::Database>::Connection,
		redis_conn: &mut RedisConnection,
		jwt_secret_key: &str,
		token: String,
	) -> Result<Self, Error> {
		if token.starts_with("patr") {
			let api_token = UserApiTokenData::parse(connection, &token).await?;
			Ok(Self::ApiToken(api_token))
		} else {
			let access_token =
				AccessTokenData::parse(token, jwt_secret_key, redis_conn)
					.await?;
			Ok(Self::AccessToken(access_token))
		}
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
	) -> &HashMap<Uuid, WorkspacePermissions> {
		match &self {
			UserAuthenticationData::AccessToken(access) => {
				&access.workspace_permissions
			}
			UserAuthenticationData::ApiToken(api) => &api.workspace_permissions,
		}
	}

	pub fn has_access_for_requested_action(
		&self,
		workspace_id: &Uuid,
		resource_id: &Uuid,
		resource_type_id: &Uuid,
		permission_required: &str,
	) -> bool {
		let workspace_permissions = self.workspace_permissions();

		let workspace_permission =
			if let Some(permission) = workspace_permissions.get(workspace_id) {
				permission
			} else {
				return false;
			};

		let allowed = {
			// Check if the resource type is allowed
			if let Some(permissions) =
				workspace_permission.resource_types.get(resource_type_id)
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
				workspace_permission.resources.get(resource_id)
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
			// Check if super admin or god is permitted
			workspace_permission.is_super_admin || {
				let god_user_id = GOD_USER_ID.get().unwrap();
				god_user_id == self.user_id()
			}
		};

		allowed
	}
}
