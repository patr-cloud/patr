use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// The identity claims a grant's scopes entitle a client to see.
///
/// One shape for both places OIDC surfaces them: the `/userinfo` response
/// body, and the id token's payload. They have to agree — OIDC Core section
/// 5.3.2 says `/userinfo` returns the same claims the id token carries — so
/// they are built once, here, rather than assembled twice.
///
/// Everything but `sub` is optional and gated on scope. A client that asked
/// for `openid` alone learns that the user exists and nothing else.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityClaims {
	/// The user, stable across every grant and every client.
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
	/// Gated on `email`, and always `true` when present — see
	/// [`build_identity_claims`].
	#[serde(skip_serializing_if = "Option::is_none")]
	pub email_verified: Option<bool>,
}

/// Just enough of a user to build their identity claims.
///
/// Deliberately not `RequestUserData`: that carries a whole permission map,
/// and the two callers here have neither the need for one nor the same way
/// of getting it — `/userinfo` has already authenticated the request, while
/// `/token` is minting an id token for a grant it has only just created.
pub struct UserIdentity<'a> {
	/// The user id, which becomes `sub`.
	pub id: Uuid,
	/// Their given name.
	pub first_name: &'a str,
	/// Their family name.
	pub last_name: &'a str,
	/// Their email address, which is also their unique identifier.
	pub email: &'a str,
}

/// Builds the identity claims for a user, keeping only what `scope` allows.
///
/// `scope` is the space-delimited set stored on the grant at consent time,
/// not whatever the client asks for now: RFC 6749 section 6 makes a refresh
/// default to the originally granted scope, so a client cannot widen its own
/// view of the user after the fact.
pub fn build_identity_claims(user: &UserIdentity<'_>, scope: &str) -> IdentityClaims {
	let granted = |wanted: &str| scope.split_whitespace().any(|scope| scope == wanted);

	IdentityClaims {
		sub: user.id.to_string(),
		name: granted("profile").then(|| format!("{} {}", user.first_name, user.last_name)),
		given_name: granted("profile").then(|| user.first_name.to_owned()),
		family_name: granted("profile").then(|| user.last_name.to_owned()),
		email: granted("email").then(|| user.email.to_owned()),
		// Asserted rather than stored: there is no column for it, and there
		// does not need to be. Sign-up is OTP-gated on the address itself and
		// the GitHub path only accepts an email GitHub has already verified,
		// so every address that reaches the `user` table is verified by
		// construction. If an unverified path is ever added, this has to
		// become a real column before it ships.
		email_verified: granted("email").then_some(true),
	}
}
