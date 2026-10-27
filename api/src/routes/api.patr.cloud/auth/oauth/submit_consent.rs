use axum::http::StatusCode;
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use models::api::auth::oauth::*;
use rand::RngExt;
use rustis::commands::{SetExpiration, StringCommands};

use super::{
	append_query_params,
	error::{OAuthError, OAuthErrorCode},
};
use crate::{
	models::oauth::types::{OAuthAuthorizationCode, OAuthAuthorizationRequest},
	prelude::*,
};

/// Records the user's decision and returns where to send the browser.
pub async fn submit_consent(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: SubmitConsentPath { request_id },
				query: (),
				headers: SubmitConsentRequestHeaders {
					authorization: _,
					user_agent,
				},
				body: SubmitConsentRequestProcessed { approved },
			},
		database,
		redis,
		client_ip,
		actor_data,
		state,
	}: AuthenticatedAppRequest<'_, SubmitConsentRequest>,
) -> Result<AppResponse<SubmitConsentRequest>, ErrorType> {
	trace!("Resolving OAuth consent request `{}`", request_id);

	// Consumed whichever way the user decides, so a decision can't be replayed.
	let parked: Option<String> = redis
		.getdel(redis::keys::oauth_authorization_request(&request_id))
		.await?;

	let Some(request) = parked
		.as_deref()
		.and_then(|payload| serde_json::from_str::<OAuthAuthorizationRequest>(payload).ok())
	else {
		debug!("No pending authorization request for `{}`", request_id);
		return Err(ErrorType::ResourceDoesNotExist);
	};

	if !state.config.oauth.clients.contains_key(&request.client_id) {
		warn!(
			"Consent submitted for client `{}`, which is no longer configured",
			request.client_id
		);
		return Err(ErrorType::ResourceDoesNotExist);
	}

	let mut params = Vec::new();

	if approved {
		// 32 random bytes: a v4 UUID would carry 122 bits of entropy, and
		// OAuth 2.1 asks for at least 128.
		let code = BASE64_URL_SAFE_NO_PAD.encode(rand::rng().random::<[u8; 32]>());

		// `auth_time` is when the user authenticated, not when they approved.
		let auth_time = query!(
			r#"
			SELECT
				created
			FROM
				user_login
			WHERE
				login_id = $1;
			"#,
			actor_data.login_id,
		)
		.fetch_one(&mut **database)
		.await?
		.created;

		let payload = OAuthAuthorizationCode {
			client_id: request.client_id.clone(),
			user_id: actor_data.id,
			approving_login_id: actor_data.login_id,
			auth_time,
			redirect_uri: request.redirect_uri.clone(),
			scopes: request.scopes.clone(),
			nonce: request.nonce.clone(),
			code_challenge: request.code_challenge.clone(),
			created_ip: client_ip,
			created_user_agent: user_agent.to_string(),
		};

		redis
			.set_with_options(
				redis::keys::oauth_authorization_code(&code),
				serde_json::to_string(&payload)?,
				None,
				SetExpiration::Ex(
					constants::OAUTH_AUTHORIZATION_CODE_VALIDITY
						.whole_seconds()
						.unsigned_abs(),
				),
			)
			.await?;

		info!(
			"User `{}` approved client `{}`",
			actor_data.id, request.client_id
		);
		params.push(("code", code));
	} else {
		info!(
			"User `{}` denied client `{}`",
			actor_data.id, request.client_id
		);
		params = OAuthError::new(OAuthErrorCode::AccessDenied, "The user denied the request")
			.as_query_params();
	}

	if let Some(oauth_state) = request.state {
		params.push(("state", oauth_state));
	}

	AppResponse::builder()
		.body(SubmitConsentResponse {
			redirect_uri: append_query_params(&request.redirect_uri, &params),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
