use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

/// Which kind of identity a row in the `identity` table describes.
///
/// Users and service accounts are both things that can hold credentials, be a
/// member of a workspace and author audit entries, so they share a supertype.
/// This discriminates the two.
///
/// It exists as a Rust type mostly so that `SELECT "user".*` keeps working:
/// both subtype tables carry a generated `identity_type` column (that is what
/// makes the composite foreign key to `identity(id, type)` unforgeable), and
/// sqlx needs a mapping for the Postgres enum to decode a wildcard select.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
	not(target_arch = "wasm32"),
	derive(sqlx::Type),
	sqlx(type_name = "IDENTITY_TYPE", rename_all = "snake_case")
)]
pub enum IdentityType {
	/// A human being, backed by a row in `"user"`.
	#[cfg_attr(not(target_arch = "wasm32"), sqlx(rename = "user"))]
	User,
	/// A non-human identity such as a runner, backed by a row in
	/// `service_account`.
	#[cfg_attr(not(target_arch = "wasm32"), sqlx(rename = "service_account"))]
	ServiceAccount,
}

impl Display for IdentityType {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::User => write!(f, "user"),
			Self::ServiceAccount => write!(f, "serviceAccount"),
		}
	}
}
