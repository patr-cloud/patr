/// The error codes an authorization request can fail with.
///
/// These are the spec's own strings (RFC 6749 section 4.1.2.1), not Patr's
/// [`ErrorType`][1] names: they go back to the client in the redirect, and
/// its library matches on them verbatim.
///
/// [1]: models::ErrorType
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuthErrorCode {
	/// The request is missing a parameter, repeats one, or is otherwise
	/// malformed.
	InvalidRequest,
	/// The response type is not one this server implements. OAuth 2.1 only
	/// defines `code`.
	UnsupportedResponseType,
	/// The requested scope is unknown, or not one this client may ask for.
	InvalidScope,
	/// The user said no.
	AccessDenied,
}

impl OAuthErrorCode {
	/// The spec string for this code.
	pub const fn as_str(self) -> &'static str {
		match self {
			Self::InvalidRequest => "invalid_request",
			Self::UnsupportedResponseType => "unsupported_response_type",
			Self::InvalidScope => "invalid_scope",
			Self::AccessDenied => "access_denied",
		}
	}
}

/// An error that goes back to the client.
#[derive(Debug, Clone)]
pub struct OAuthError {
	/// Which error this is.
	pub code: OAuthErrorCode,
	/// A human-readable explanation. Shown to developers, never to end
	/// users, so it should say what is actually wrong.
	pub description: String,
}

impl OAuthError {
	/// Builds an error with the given code and description.
	pub fn new(code: OAuthErrorCode, description: impl Into<String>) -> Self {
		Self {
			code,
			description: description.into(),
		}
	}

	/// The `error` / `error_description` pair as query parameters, for
	/// redirecting the failure back to the client.
	pub fn as_query_params(&self) -> Vec<(&'static str, String)> {
		vec![
			("error", self.code.as_str().to_owned()),
			("error_description", self.description.clone()),
		]
	}
}
