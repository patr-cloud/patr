use axum::{
	Json,
	http::{StatusCode, header::WWW_AUTHENTICATE},
	response::{IntoResponse, Response},
};

use crate::prelude::*;

/// The error codes an OAuth request goes back to the client with (RFC 6749
/// sections 4.1.2.1 and 5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuthErrorCode {
	/// A parameter is missing or malformed.
	InvalidRequest,
	/// The client is unknown, or failed to authenticate.
	InvalidClient,
	/// The code or refresh token is invalid, expired, used, or another
	/// client's.
	InvalidGrant,
	/// `grant_type` is one this server doesn't implement.
	UnsupportedGrantType,
	/// `response_type` isn't `code`.
	UnsupportedResponseType,
	/// A scope is unknown, or one this client may not request.
	InvalidScope,
	/// The user said no.
	AccessDenied,
	/// Something broke on our side.
	ServerError,
	/// The caller is rate limited.
	TemporarilyUnavailable,
}

impl OAuthErrorCode {
	/// The spec string for this code.
	pub const fn as_str(self) -> &'static str {
		match self {
			Self::InvalidRequest => "invalid_request",
			Self::InvalidClient => "invalid_client",
			Self::InvalidGrant => "invalid_grant",
			Self::UnsupportedGrantType => "unsupported_grant_type",
			Self::UnsupportedResponseType => "unsupported_response_type",
			Self::InvalidScope => "invalid_scope",
			Self::AccessDenied => "access_denied",
			Self::ServerError => "server_error",
			Self::TemporarilyUnavailable => "temporarily_unavailable",
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

	/// Logs the cause and turns it into an opaque `server_error`.
	pub fn server_error(err: impl std::fmt::Display) -> Self {
		error!("Internal server error occured: {err}");
		Self::new(OAuthErrorCode::ServerError, "internal error")
	}

	/// The error as `error` and `error_description` query parameters.
	pub fn as_query_params(&self) -> Vec<(&'static str, String)> {
		vec![
			("error", self.code.as_str().to_owned()),
			("error_description", self.description.clone()),
		]
	}
}

impl From<sqlx::Error> for OAuthError {
	fn from(err: sqlx::Error) -> Self {
		Self::server_error(err)
	}
}

impl From<rustis::Error> for OAuthError {
	fn from(err: rustis::Error) -> Self {
		Self::server_error(err)
	}
}

impl From<serde_json::Error> for OAuthError {
	fn from(err: serde_json::Error) -> Self {
		Self::server_error(err)
	}
}

impl IntoResponse for OAuthError {
	fn into_response(self) -> Response {
		let body = Json(serde_json::json!({
			"error": self.code.as_str(),
			"error_description": self.description,
		}));

		match self.code {
			// RFC 6749 section 5.2: a client that failed to authenticate is
			// told how it should have.
			OAuthErrorCode::InvalidClient => (
				StatusCode::UNAUTHORIZED,
				[(WWW_AUTHENTICATE, r#"Basic realm="patr", charset="UTF-8""#)],
				body,
			)
				.into_response(),
			OAuthErrorCode::ServerError => {
				(StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
			}
			OAuthErrorCode::TemporarilyUnavailable => {
				(StatusCode::TOO_MANY_REQUESTS, body).into_response()
			}
			_ => (StatusCode::BAD_REQUEST, body).into_response(),
		}
	}
}
