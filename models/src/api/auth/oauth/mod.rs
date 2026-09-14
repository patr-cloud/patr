//! Patr's own OAuth 2.1 / OpenID Connect provider.
//!
//! The protocol endpoints themselves — `/authorize`, `/token`, `/userinfo`
//! and friends — are **not** declared here. They speak the spec's wire
//! format: form-encoded requests, bare JSON responses, and `error` /
//! `error_description` bodies, none of which survive Patr's success and
//! error envelopes. They live as plain axum routes in the API crate
//! instead.
//!
//! What is declared here is the pair of endpoints the *dashboard* calls to
//! render and resolve a consent screen. Those are ordinary first-party JSON
//! APIs, so they keep the macro, the layer stack, and the generated
//! TypeScript bindings.

/// Reads the pending authorization request behind a consent screen.
mod get_consent_request;
/// Records the user's decision and returns where to send the browser.
mod submit_consent;

use serde::{Deserialize, Serialize};

pub use self::{get_consent_request::*, submit_consent::*};

/// An identity scope being requested, with copy the consent screen can show
/// a user directly.
///
/// The descriptions are built server-side so that the wording of what a user
/// is agreeing to lives in one place, rather than being reconstructed by
/// every client that renders a consent screen.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct OAuthScopeInfo {
	/// The scope string, as the client requested it.
	pub scope: String,
	/// A short label, e.g. "Your profile".
	pub title: String,
	/// A sentence explaining what granting it allows.
	pub description: String,
}
