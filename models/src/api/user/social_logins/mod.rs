use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::api::auth::SocialLoginProvider;

/// The endpoint to complete the social login connect flow
mod callback;
/// The endpoint to initiate connecting a social login identity to the current
/// user
mod connect;
/// The endpoint to disconnect a social-login provider from the current user
mod disconnect;
/// The endpoint to list the social logins linked to the current user
mod list;

pub use self::{callback::*, connect::*, disconnect::*, list::*};

/// One social-login row linked to the caller's account.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct LinkedSocialLogin {
	/// The provider this row links to (currently only `github`).
	pub provider: SocialLoginProvider,
	/// When the link was created.
	#[ts(type = "string")]
	pub linked_at: OffsetDateTime,
}
