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
}

impl CreateApiTokenInfo {
	pub fn new() -> Self {
		Self {
			name: None,
			token_nbf: Some(OffsetDateTime::now_utc()),
			token_exp: None,
		}
	}
}
