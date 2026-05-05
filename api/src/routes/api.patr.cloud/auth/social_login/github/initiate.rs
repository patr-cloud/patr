use axum::http::StatusCode;
use models::api::auth::*;
use rustis::commands::StringCommands;

use crate::{models::social_login::GithubStatePayload, prelude::*};

/// CSRF state token validity: 10 minutes
const GITHUB_STATE_TTL_SECS: u64 = 600;

/// `POST /auth/social-login/{provider}`
///
/// Generates a CSRF state UUID, stores it in Redis for 10 minutes, and
/// returns the full provider authorization URL that the frontend should
/// redirect to.
pub async fn social_login_initiate(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: SocialLoginInitiatePath { provider },
				query: (),
				headers: (),
				body: SocialLoginInitiateRequestProcessed,
			},
		database: _,
		redis,
		client_ip: _,
		state,
	}: AppRequest<'_, SocialLoginInitiateRequest>,
) -> Result<AppResponse<SocialLoginInitiateRequest>, ErrorType> {
	trace!("Initiating {provider} OAuth flow");

	#[expect(irrefutable_let_patterns)]
	let SocialLoginProvider::GitHub = provider else {
		return Err(ErrorType::SocialLoginFailed);
	};

	let oauth_state_token = Uuid::new_v4().to_string();

	redis
		.setex(
			redis::keys::social_login_state(&provider, &oauth_state_token),
			GITHUB_STATE_TTL_SECS,
			serde_json::to_string(&GithubStatePayload::Anonymous)?,
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
		.body(SocialLoginInitiateResponse { authorize_url })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
