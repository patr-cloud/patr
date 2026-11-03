use axum::{
	Json,
	extract::State,
	http::{HeaderMap, StatusCode, header::WWW_AUTHENTICATE},
	response::{IntoResponse, Response},
};
use headers::HeaderMapExt;
use models::{ActorData, utils::BearerToken};

use crate::{
	models::{
		oauth::identity::{UserIdentity, build_identity_claims},
		permissions,
	},
	prelude::*,
	utils::extractors::ClientIP,
};

/// `GET`/`POST /auth/oauth/userinfo`: the identity claims the grant's scopes
/// allow (OIDC Core section 5.3).
pub async fn userinfo(
	State(state): State<AppState>,
	ClientIP(client_ip): ClientIP,
	headers: HeaderMap,
) -> Response {
	if let Err(error) = super::enforce_rate_limit(&state, client_ip).await {
		return error.into_response();
	}

	let Some(BearerToken(token)) = headers.typed_get::<BearerToken>() else {
		return unauthorized("invalid_request", "a bearer token is required");
	};

	let mut database = match state.database.acquire().await {
		Ok(connection) => connection,
		Err(err) => {
			error!("Failed to acquire database connection: {}", err);
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		}
	};

	let Ok(actor_data) = permissions::authenticate(
		&mut database,
		&mut state.redis.clone(),
		&state.config,
		client_ip,
		token.token(),
		&[ActorClientType::UserLogin(UserLoginType::OAuthLogin)],
	)
	.await
	else {
		return unauthorized(
			"invalid_token",
			"the access token is invalid or has expired",
		);
	};

	let ActorData::User {
		email,
		first_name,
		last_name,
		..
	} = &actor_data.actor
	else {
		return unauthorized("invalid_token", "the access token is not a user's");
	};

	let scope = match query!(
		r#"
		SELECT
			scope
		FROM
			oauth_login
		WHERE
			login_id = $1;
		"#,
		actor_data.login_id,
	)
	.fetch_one(&mut *database)
	.await
	{
		Ok(row) => row.scope,
		Err(err) => {
			error!("Failed to read the grant's scope: {}", err);
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		}
	};

	Json(build_identity_claims(
		&UserIdentity {
			id: actor_data.id,
			first_name,
			last_name,
			email,
		},
		&scope,
	))
	.into_response()
}

/// A 401 with the challenge RFC 6750 section 3 defines, whose `error` tells a
/// client whether refreshing will help.
fn unauthorized(error: &str, description: &str) -> Response {
	(
		StatusCode::UNAUTHORIZED,
		[(
			WWW_AUTHENTICATE,
			format!(r#"Bearer error="{error}", error_description="{description}""#),
		)],
		Json(serde_json::json!({
			"error": error,
			"error_description": description,
		})),
	)
		.into_response()
}
