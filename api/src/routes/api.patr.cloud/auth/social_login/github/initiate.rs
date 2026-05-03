use axum::http::StatusCode;
use models::api::auth::*;
use rustis::commands::StringCommands;

use crate::{prelude::*, redis::keys as redis_keys, utils::cloudflare::validate_turnstile_token};

/// CSRF state token validity: 10 minutes
const GITHUB_STATE_TTL_SECS: u64 = 600;

/// `POST /auth/social-login/github`
///
/// Validates the Cloudflare Turnstile token (reused from the login or signup
/// page that surfaced the GitHub button), generates a CSRF state UUID, stores
/// it in Redis for 10 minutes, and returns the full GitHub authorization URL
/// that the frontend should redirect to.
pub async fn github_oauth_initiate(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: GithubOAuthInitiatePath {},
				query: (),
				headers: (),
				body: GithubOAuthInitiateRequestProcessed { cf_turnstile_token },
			},
		redis,
		client_ip,
		state,
		..
	}: AppRequest<'_, GithubOAuthInitiateRequest>,
) -> Result<AppResponse<GithubOAuthInitiateRequest>, ErrorType> {
	trace!("Validating Cloudflare Turnstile token for GitHub OAuth initiate");
	let cf_turnstile_response = validate_turnstile_token(
		&state.config.cloudflare.turnstile_secret,
		&cf_turnstile_token,
		Some(client_ip),
	)
	.await
	.inspect_err(|err| {
		error!("Error verifying Cloudflare Turnstile token: `{}`", err);
	})?;

	if !cf_turnstile_response.success {
		return Err(ErrorType::TurnstileVerificationFailed);
	}

	// The GitHub button is surfaced on `/login` and `/sign-up`; both pages'
	// Turnstile widgets are valid sources of a token for this endpoint.
	if !cfg!(debug_assertions) &&
		!matches!(cf_turnstile_response.action.as_str(), "login" | "sign-up")
	{
		return Err(ErrorType::TurnstileVerificationActionMismatch);
	}

	trace!("Initiating GitHub OAuth flow");

	let oauth_state_token = Uuid::new_v4().to_string();

	redis
		.setex(
			redis_keys::social_login_state(&OAuthProvider::Github, &oauth_state_token),
			GITHUB_STATE_TTL_SECS,
			"1",
		)
		.await
		.inspect_err(|err| {
			error!("Error storing GitHub OAuth state in Redis: {err}");
		})?;

	let mut authorize_url = reqwest::Url::parse("https://github.com/login/oauth/authorize")
		.expect("static GitHub OAuth URL is valid");
	authorize_url
		.query_pairs_mut()
		.append_pair("client_id", &state.config.social_login.github.client_id)
		.append_pair(
			"redirect_uri",
			&state.config.social_login.github.callback_url,
		)
		.append_pair("scope", "read:user user:email")
		.append_pair("state", &oauth_state_token);
	let authorize_url = authorize_url.to_string();

	AppResponse::builder()
		.body(GithubOAuthInitiateResponse { authorize_url })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
