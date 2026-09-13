use axum::{
	Json,
	http::StatusCode,
	response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::prelude::*;

/// The error codes an authorization or token request can fail with.
///
/// These are the spec's own strings (RFC 6749 sections 4.1.2.1 and 5.2, plus
/// RFC 6750), not Patr's [`ErrorType`][1] names. A client library matches on
/// them verbatim, so they are deliberately not routed through Patr's usual
/// error envelope.
///
/// [1]: models::ErrorType
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthErrorCode {
	/// The request is missing a parameter, repeats one, or is otherwise
	/// malformed.
	InvalidRequest,
	/// The client is not known, or failed to authenticate.
	InvalidClient,
	/// The authorization code or refresh token is invalid, expired, revoked,
	/// already used, or was issued to another client.
	InvalidGrant,
	/// The grant type is not one this server implements.
	UnsupportedGrantType,
	/// RFC 7009 section 2.2.1: the `token_type_hint` named a kind of token
	/// this server does not issue, so the request cannot be honoured.
	UnsupportedTokenType,
	/// The response type is not one this server implements. OAuth 2.1 only
	/// defines `code`.
	UnsupportedResponseType,
	/// The requested scope is unknown, or not one this client may ask for.
	InvalidScope,
	/// The user said no.
	AccessDenied,
	/// Something broke on our side.
	ServerError,
	/// The caller is being rate limited.
	///
	/// RFC 6749 registers `temporarily_unavailable` for a server that cannot
	/// handle the request right now, which is what a tripped rate limit is.
	/// Rendered as 429 rather than the 503 the spec suggests, because the
	/// cause is the caller's rate rather than our health.
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
			Self::UnsupportedTokenType => "unsupported_token_type",
			Self::UnsupportedResponseType => "unsupported_response_type",
			Self::InvalidScope => "invalid_scope",
			Self::AccessDenied => "access_denied",
			Self::ServerError => "server_error",
			Self::TemporarilyUnavailable => "temporarily_unavailable",
		}
	}

	/// The status code this error is reported with when it is returned in a
	/// response body rather than a redirect.
	const fn status_code(self) -> StatusCode {
		match self {
			// RFC 6749 section 5.2: a client that fails to authenticate gets
			// 401, so it can tell "your credentials are wrong" apart from
			// "your request was wrong".
			Self::InvalidClient => StatusCode::UNAUTHORIZED,
			Self::ServerError => StatusCode::INTERNAL_SERVER_ERROR,
			Self::TemporarilyUnavailable => StatusCode::TOO_MANY_REQUESTS,
			_ => StatusCode::BAD_REQUEST,
		}
	}
}

/// An error from one of the OAuth protocol endpoints. Serializes as the body
/// RFC 6749 section 5.2 defines; `/authorize` sends it back to the client as
/// query parameters instead.
#[derive(Debug, Clone, Serialize)]
pub struct OAuthError {
	/// Which error this is.
	#[serde(rename = "error")]
	pub code: OAuthErrorCode,
	/// A human-readable explanation. Shown to developers, never to end
	/// users, so it should say what is actually wrong.
	#[serde(rename = "error_description")]
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

	/// The `error` / `error_description` pair as query parameters, for
	/// redirecting the failure back to the client.
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
		let status = self.code.status_code();

		// RFC 6749 section 5.2: a client that failed to authenticate must be
		// told how it was supposed to. Basic is what we accept alongside
		// credentials in the body.
		if status == StatusCode::UNAUTHORIZED {
			return (
				status,
				[(
					axum::http::header::WWW_AUTHENTICATE,
					r#"Basic realm="patr", charset="UTF-8""#,
				)],
				Json(self),
			)
				.into_response();
		}

		(status, Json(self)).into_response()
	}
}
