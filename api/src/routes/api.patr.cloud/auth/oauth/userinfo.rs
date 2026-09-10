use axum::{
	Json,
	extract::State,
	http::{HeaderMap, StatusCode, header},
	response::{IntoResponse, Response},
};

use crate::{
	models::{
		oauth::identity::{UserIdentity, build_identity_claims},
		permissions,
	},
	prelude::*,
	utils::extractors::ClientIP,
};

/// `GET`/`POST /auth/oauth/userinfo` — the OIDC UserInfo endpoint.
///
/// Returns the identity claims the grant's scopes allow, and nothing about
/// the user's workspaces or resources: this answers "who is this", which is
/// all a relying party needs to finish a login. Grafana's `api_url` points
/// here, so signing in does not work without it.
///
/// Mounted for both methods because OIDC Core section 5.3 requires both, and
/// outside `declare_api_endpoint!` because the response is a bare claims
/// object — a client library deserialises it directly, with no room for
/// Patr's success envelope around it.
pub async fn userinfo(
	State(state): State<AppState>,
	ClientIP(client_ip): ClientIP,
	headers: HeaderMap,
) -> Response {
	// Authenticated, but still worth throttling: every call here validates a
	// signature and reads the grant, so an unthrottled one is a cheap way to
	// make the API do expensive work.
	if let Err(error) = super::enforce_rate_limit(&state, client_ip).await {
		return error.into_response();
	}

	let Some(token) = bearer_token(&headers) else {
		return unauthorized("invalid_request", "a bearer token is required");
	};

	let mut database = match state.database.begin().await {
		Ok(connection) => connection,
		Err(err) => {
			error!("Error starting a transaction for /userinfo: {}", err);
			return unauthorized("invalid_token", "the access token could not be validated");
		}
	};
	let mut redis = state.redis.clone();

	let grant =
		permissions::get_grant_context(&mut database, &mut redis, &state.config, &token).await;

	// Nothing here writes, so the transaction only ever needs releasing.
	_ = database.rollback().await;

	let Ok(grant) = grant else {
		return unauthorized(
			"invalid_token",
			"the access token is invalid or has expired",
		);
	};

	let identity = UserIdentity {
		id: grant.user_data.id,
		first_name: &grant.user_data.first_name,
		last_name: &grant.user_data.last_name,
		email: &grant.user_data.email,
	};

	Json(build_identity_claims(&identity, &grant.scope)).into_response()
}

/// Pulls the token out of an `Authorization: Bearer` header.
///
/// The header is the only form accepted. OIDC Core section 5.3.1 permits a
/// form-encoded `access_token` body parameter too, but that puts a live
/// credential somewhere it gets logged, and every client that matters sends
/// the header.
fn bearer_token(headers: &HeaderMap) -> Option<String> {
	headers
		.get(header::AUTHORIZATION)?
		.to_str()
		.ok()?
		.strip_prefix("Bearer ")
		.map(str::trim)
		.filter(|token| !token.is_empty())
		.map(ToOwned::to_owned)
}

/// A 401 carrying the `WWW-Authenticate` challenge RFC 6750 section 3 defines.
///
/// The challenge is not decoration: a client library reads `error` from it to
/// tell "this token expired, refresh and retry" apart from "this request was
/// malformed, retrying will not help". Without it Grafana treats an expired
/// token as a hard failure.
fn unauthorized(error: &str, description: &str) -> Response {
	(
		StatusCode::UNAUTHORIZED,
		[(
			header::WWW_AUTHENTICATE,
			format!(r#"Bearer error="{error}", error_description="{description}""#),
		)],
		Json(serde_json::json!({
			"error": error,
			"error_description": description,
		})),
	)
		.into_response()
}
