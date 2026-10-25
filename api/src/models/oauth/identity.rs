use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// The identity claims a grant's scopes let a client see, as `/userinfo`
/// returns them and the id token carries them (OIDC Core section 5.3.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityClaims {
	/// The user.
	pub sub: String,
	/// Gated on `profile`.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,
	/// Gated on `profile`.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub given_name: Option<String>,
	/// Gated on `profile`.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub family_name: Option<String>,
	/// Gated on `email`.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub email: Option<String>,
	/// Gated on `email`, and always `true`: sign-up is OTP-gated on the
	/// address, and GitHub sign-in only takes a verified one.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub email_verified: Option<bool>,
}

/// Just enough of a user to build their identity claims.
pub struct UserIdentity<'a> {
	/// The user id, which becomes `sub`.
	pub id: Uuid,
	/// Their given name.
	pub first_name: &'a str,
	/// Their family name.
	pub last_name: &'a str,
	/// Their email address.
	pub email: &'a str,
}

/// Builds a user's identity claims, keeping only what the grant's `scope`
/// allows.
pub fn build_identity_claims(user: &UserIdentity<'_>, scope: &str) -> IdentityClaims {
	let granted = |wanted: &str| scope.split_whitespace().any(|scope| scope == wanted);

	IdentityClaims {
		sub: user.id.to_string(),
		name: granted("profile").then(|| format!("{} {}", user.first_name, user.last_name)),
		given_name: granted("profile").then(|| user.first_name.to_owned()),
		family_name: granted("profile").then(|| user.last_name.to_owned()),
		email: granted("email").then(|| user.email.to_owned()),
		email_verified: granted("email").then_some(true),
	}
}
