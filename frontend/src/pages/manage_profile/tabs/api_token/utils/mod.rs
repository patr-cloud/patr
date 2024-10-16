use std::collections::BTreeMap;

use leptos::prelude::*;
use models::{api::user::UserApiToken, prelude::*, rbac::WorkspacePermission};
use time::OffsetDateTime;

/// The Api Token Permissions
#[derive(Clone, Debug)]
pub struct ApiTokenPermissions(pub RwSignal<Option<BTreeMap<Uuid, WorkspacePermission>>>);

/// Context for the Edit API Token Page
#[derive(Copy, Clone)]
pub struct ApiTokenInfo(pub RwSignal<Option<WithId<UserApiToken>>>);

/// The Api Token Info
#[derive(Clone, Debug)]
pub struct CreateApiTokenInfo {
	/// The name of the Api token
	pub name: Option<String>,
	/// When the token will be valid from
	pub token_nbf: Option<OffsetDateTime>,
	/// When the token will be valid till
	pub token_exp: Option<OffsetDateTime>,
	/// The permissions of the Api token
	pub permission: BTreeMap<Uuid, WorkspacePermission>,
}

impl CreateApiTokenInfo {
	/// Convert the ApiTokenInfo to a UserApiToken
	pub fn convert_to_user_api_token(&self) -> Option<UserApiToken> {
		let name = self.name.clone()?;

		Some(UserApiToken {
			name,
			permissions: self.permission.clone(),
			token_nbf: self.token_nbf.clone(),
			token_exp: self.token_exp.clone(),
			allowed_ips: None,
			created: OffsetDateTime::UNIX_EPOCH,
		})
	}

	pub const fn new() -> Self {
		Self {
			name: None,
			token_nbf: None,
			token_exp: None,
			permission: BTreeMap::new(),
		}
	}
}
