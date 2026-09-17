//! Patr's own OAuth 2.1 / OpenID Connect provider.
//!
//! Only `/authorize` and the two consent endpoints are declared here.
//! `/authorize` answers with a redirect, and the consent endpoints are
//! ordinary first-party JSON the dashboard calls, so all three fit the
//! macro. The back-channel endpoints — `/token`, `/userinfo` and friends —
//! do not: a client library parses their bodies, which must be the spec's
//! bare JSON with `error` / `error_description` on failure, and neither
//! survives Patr's envelopes. Those live as plain axum routes in the API
//! crate.

/// Where a client sends the browser to start a login.
mod authorize;
/// Reads the pending authorization request behind a consent screen.
mod get_consent_request;
/// Records the user's decision and returns where to send the browser.
mod submit_consent;

pub use self::{authorize::*, get_consent_request::*, submit_consent::*};
