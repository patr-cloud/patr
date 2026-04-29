use axum::http::StatusCode;
use models::api::auth::*;
use rustis::commands::StringCommands;

use crate::{prelude::*, redis::keys as redis_keys};

/// CSRF state token validity: 10 minutes
const GITHUB_STATE_TTL_SECS: u64 = 600;

/// `GET /auth/social-login/github`
///
/// Generates a CSRF state UUID, stores it in Redis for 10 minutes, and returns
/// the full GitHub authorization URL that the frontend should redirect to.
pub async fn github_oauth_initiate(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: GithubOAuthInitiatePath,
				query: (),
				headers: (),
				body: GithubOAuthInitiateRequestProcessed,
			},
		redis,
		state,
		..
	}: AppRequest<'_, GithubOAuthInitiateRequest>,
) -> Result<AppResponse<GithubOAuthInitiateRequest>, ErrorType> {
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
