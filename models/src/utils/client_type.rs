use std::fmt::Display;

use serde::{Deserialize, Serialize};
use strum::EnumDiscriminants;

use crate::UserLoginType;

/// Which kind of credential a request authenticated with. This decides which
/// endpoints the caller may reach.
///
/// Mirrors the database's two levels: `ACTOR_CLIENT_TYPE` separates a user
/// login from a service account ([`ActorClientTypeDiscriminant`] decodes it),
/// and `USER_LOGIN_TYPE` beneath it separates a web session from an API token
/// ([`UserLoginType`]).
///
/// - [`UserLogin`][Self::UserLogin]`(`[`WebLogin`][UserLoginType::WebLogin]`)`: the web dashboard,
///   authenticated via JWT.
/// - [`UserLogin`][Self::UserLogin]`(`[`ApiToken`][UserLoginType::ApiToken]`)`: third-party
///   applications, authenticated via user API tokens (`patrv1.*`).
/// - [`ServiceAccount`][Self::ServiceAccount]: non-human identities like runners, authenticated via
///   service account tokens (`patrv1.*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumDiscriminants)]
#[serde(rename_all = "camelCase")]
#[strum_discriminants(
	name(ActorClientTypeDiscriminant),
	cfg_attr(
		not(target_arch = "wasm32"),
		derive(sqlx::Type),
		sqlx(type_name = "ACTOR_CLIENT_TYPE", rename_all = "snake_case"),
	),
	doc = "The database's `ACTOR_CLIENT_TYPE`: a user login or a service account"
)]
pub enum ActorClientType {
	/// One of a user's logins: a web session or an API token
	UserLogin(UserLoginType),
	/// A service account's token
	ServiceAccount,
}

impl Display for ActorClientType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::UserLogin(UserLoginType::WebLogin) => write!(f, "WebLogin"),
			Self::UserLogin(UserLoginType::ApiToken) => write!(f, "ApiToken"),
			Self::ServiceAccount => write!(f, "ServiceAccount"),
		}
	}
}
