use axum::{
	Json,
	http::StatusCode,
	response::{IntoResponse, Redirect, Response},
};
use serde::Serialize;

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
	/// The spec string for this code, for building a redirect query.
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

/// An error from one of the OAuth protocol endpoints.
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

/// The body of an error response, as RFC 6749 section 5.2 defines it.
#[derive(Debug, Serialize)]
struct OAuthErrorBody {
	/// The error code.
	error: &'static str,
	/// The human-readable explanation.
	error_description: String,
}

impl IntoResponse for OAuthError {
	fn into_response(self) -> Response {
		let status = self.code.status_code();
		let body = Json(OAuthErrorBody {
			error: self.code.as_str(),
			error_description: self.description,
		});

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
				body,
			)
				.into_response();
		}

		(status, body).into_response()
	}
}

/// How `/authorize` failed.
///
/// The split matters and is easy to get wrong. RFC 6749 section 4.1.2.1 says
/// that when the client is unknown, or its `redirect_uri` does not match one
/// it registered, the server **must not** redirect — the URI is unverified,
/// so bouncing the browser to it would turn the authorization endpoint into
/// an open redirect for anyone who can guess a client id. Every other error
/// is reported by redirecting back to the (now trusted) URI, because that is
/// the only way the client learns what happened.
#[derive(Debug)]
pub enum AuthorizeError {
	/// The redirect target could not be trusted, so the failure is rendered
	/// rather than redirected.
	Untrusted(OAuthError),
	/// The redirect target was validated, so the failure goes back to it.
	Redirect {
		/// The validated redirect URI.
		redirect_uri: String,
		/// The client's `state`, echoed back verbatim when it sent one.
		state: Option<String>,
		/// What went wrong.
		error: OAuthError,
	},
}

impl AuthorizeError {
	/// An error that must not be redirected anywhere.
	pub fn untrusted(code: OAuthErrorCode, description: impl Into<String>) -> Self {
		Self::Untrusted(OAuthError::new(code, description))
	}

	/// An error that goes back to the client's validated redirect URI.
	pub fn redirect(
		redirect_uri: impl Into<String>,
		state: Option<String>,
		code: OAuthErrorCode,
		description: impl Into<String>,
	) -> Self {
		Self::Redirect {
			redirect_uri: redirect_uri.into(),
			state,
			error: OAuthError::new(code, description),
		}
	}
}

impl IntoResponse for AuthorizeError {
	fn into_response(self) -> Response {
		match self {
			// Always a 400, and never a `WWW-Authenticate` challenge, even
			// for `invalid_client`. The 401-with-Basic rule in RFC 6749
			// section 5.2 is about a *client* failing to authenticate at the
			// token endpoint; nothing authenticates here. The recipient of
			// this response is a browser, and a 401 carrying a Basic
			// challenge would make it pop a native credential prompt at the
			// user — for a client they have never heard of.
			Self::Untrusted(error) => (
				StatusCode::BAD_REQUEST,
				Json(OAuthErrorBody {
					error: error.code.as_str(),
					error_description: error.description,
				}),
			)
				.into_response(),
			Self::Redirect {
				redirect_uri,
				state,
				error,
			} => {
				let mut params = error.as_query_params();
				// Echoed verbatim, and omitted entirely when the client sent
				// none — `state=` with an empty value is not the same thing,
				// and a client comparing it against what it stored would
				// reject the response.
				if let Some(state) = state {
					params.push(("state", state));
				}

				Redirect::to(&super::append_query_params(&redirect_uri, &params)).into_response()
			}
		}
	}
}
