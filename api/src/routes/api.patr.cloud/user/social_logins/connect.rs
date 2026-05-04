use axum::http::StatusCode;
use models::api::{auth::SocialLoginProvider, user::*};
use rustis::commands::StringCommands;

use crate::{models::social_login::GithubStatePayload, prelude::*};

pub async fn connect_social_login_initiate(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ConnectSocialLoginInitiatePath { provider },
				query: (),
				headers:
					ConnectSocialLoginInitiateRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ConnectSocialLoginInitiateRequestProcessed,
			},
		redis,
		user_data,
		state,
		..
	}: AuthenticatedAppRequest<'_, ConnectSocialLoginInitiateRequest>,
) -> Result<AppResponse<ConnectSocialLoginInitiateRequest>, ErrorType> {
	trace!("Initiating GitHub connect flow for user {}", user_data.id);

	let oauth_state_token = Uuid::new_v4().to_string();
	let payload = serde_json::to_string(&GithubStatePayload::Authenticated {
		user_id: user_data.id,
	})
	.map_err(ErrorType::server_error)?;

	#[expect(irrefutable_let_patterns)]
	let SocialLoginProvider::GitHub = provider else {
		return Err(ErrorType::SocialLoginFailed);
	};

	// State token validity: 10 minutes.
	redis
		.setex(
			redis::keys::social_login_state(&provider, &oauth_state_token),
			600,
			payload,
		)
		.await
		.inspect_err(|err| {
			error!("Error storing GitHub connect state in Redis: {err}");
		})?;

	let mut authorize_url = reqwest::Url::parse("https://github.com/login/oauth/authorize")
		.expect("static GitHub OAuth URL is valid");
	authorize_url
		.query_pairs_mut()
		.append_pair("client_id", &state.config.social_login.github.client_id)
		.append_pair(
			"redirect_uri",
			&state.config.social_login.github.connect_callback_url,
		)
		.append_pair("scope", "read:user user:email")
		.append_pair("state", &oauth_state_token);

	AppResponse::builder()
		.body(ConnectSocialLoginInitiateResponse {
			authorize_url: authorize_url.to_string(),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
