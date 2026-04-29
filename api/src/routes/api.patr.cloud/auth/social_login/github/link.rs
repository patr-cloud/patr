use axum::http::StatusCode;
use models::api::auth::*;
use rustis::commands::StringCommands;
use time::OffsetDateTime;

use super::{GithubLinkPayload, create_session};
use crate::{prelude::*, redis::keys as redis_keys};

/// `POST /auth/social-login/github/link`
///
/// Confirms linking a GitHub account to an existing Patr account. The
/// `link_token` was issued by the callback handler.
pub async fn github_oauth_link(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: GithubOAuthLinkPath,
				query: (),
				headers: GithubOAuthLinkRequestHeaders { user_agent },
				body: GithubOAuthLinkRequestProcessed { link_token },
			},
		database,
		redis,
		client_ip,
		mut state,
	}: AppRequest<'_, GithubOAuthLinkRequest>,
) -> Result<AppResponse<GithubOAuthLinkRequest>, ErrorType> {
	trace!("Processing GitHub OAuth link confirmation");

	// Atomically fetch-and-consume the link token. `GETDEL` ensures two
	// concurrent requests with the same token cannot both observe it as valid.
	let link_key = redis_keys::social_login_link(&OAuthProvider::Github, &link_token);
	let raw = redis
		.getdel::<Option<String>>(&link_key)
		.await
		.inspect_err(|err| error!("Redis error consuming link token: {err}"))?
		.ok_or(ErrorType::GithubOAuthFailed)?;

	let payload: GithubLinkPayload = serde_json::from_str(&raw).map_err(ErrorType::server_error)?;

	// Insert the GitHub link (idempotent)
	query!(
		r#"
		INSERT INTO user_social_login(user_id, provider, external_id, linked_at)
		VALUES ($1, 'github', $2, $3)
		ON CONFLICT (provider, external_id) DO NOTHING;
		"#,
		payload.user_id as _,
		payload.external_id,
		OffsetDateTime::now_utc(),
	)
	.execute(&mut **database)
	.await?;

	let (access_token, refresh_token) = create_session(
		database,
		&mut state,
		redis,
		client_ip,
		user_agent.to_string(),
		payload.user_id,
	)
	.await?;

	AppResponse::builder()
		.body(GithubOAuthLinkResponse {
			access_token,
			refresh_token,
		})
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
