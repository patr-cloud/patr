/// The error codes an authorization request goes back to the client with
/// (RFC 6749 section 4.1.2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuthErrorCode {
	/// A parameter is missing or malformed.
	InvalidRequest,
	/// `response_type` isn't `code`.
	UnsupportedResponseType,
	/// A scope is unknown, or one this client may not request.
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
	/// What's wrong, for the client's developer.
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

	/// The error as `error` and `error_description` query parameters.
	pub fn as_query_params(&self) -> Vec<(&'static str, String)> {
		vec![
			("error", self.code.as_str().to_owned()),
			("error_description", self.description.clone()),
		]
	}
}
