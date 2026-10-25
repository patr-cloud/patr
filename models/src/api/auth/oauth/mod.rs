//! Patr's own OAuth 2.1 / OpenID Connect provider.

/// Where a client sends the browser to start a login.
mod authorize;
/// Reads the pending authorization request behind a consent screen.
mod get_consent_request;
/// Records the user's decision and returns where to send the browser.
mod submit_consent;

pub use self::{authorize::*, get_consent_request::*, submit_consent::*};
