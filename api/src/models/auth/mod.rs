mod access_token_data;
mod email_validate;
mod oauth;
mod user_api_token_data;

use std::{collections::BTreeMap, net::IpAddr};

use api_models::{
	models::workspace::{ResourcePermissionType, WorkspacePermission},
	utils::Uuid,
};
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
		permission_required: &str,
	) -> bool {
		is_user_action_authorized(
			self.workspace_permissions(),
			self.user_id(),
			workspace_id,
			permission_required,
			resource_id,
		)
	}
}

pub fn is_user_action_authorized(
	user_permissions: &BTreeMap<Uuid, WorkspacePermission>,
	user_id: &Uuid,
	requested_workspace: &Uuid,
	requested_permission: &str,
	requested_resource: &Uuid,
) -> bool {
	if user_id == GOD_USER_ID.get().unwrap() {
		// allow all operations on all workspace for god user
		return true;
	}

	let Some(workspace_permission) =
		user_permissions.get(requested_workspace)
	else {
		// user don't have any permission on given workspace
		return false;
	};

	match workspace_permission {
		WorkspacePermission::SuperAdmin => {
			// allow all operations on given workspace for super admin
			true
		}
		WorkspacePermission::Member(workspace_member_permissions) => {
			let permission_required = rbac::PERMISSIONS
				.get()
				.unwrap()
				.get(&(*requested_permission).to_string())
				.unwrap();

			let Some(resource_permission_type) = workspace_member_permissions.get(permission_required) else {
				// user don't have required permission 
				return false;
			};

			match resource_permission_type {
				ResourcePermissionType::Include(resource_ids) => {
					resource_ids.contains(requested_resource)
				}
				ResourcePermissionType::Exclude(resource_ids) => {
					!resource_ids.contains(requested_resource)
				}
			}
		}
	}
}
