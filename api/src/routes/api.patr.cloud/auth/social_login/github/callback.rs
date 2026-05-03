use axum::http::StatusCode;
use models::api::auth::*;
use rustis::commands::StringCommands;
use serde::Deserialize;

use super::{
	GITHUB_LINK_TTL_SECS,
	GITHUB_SETUP_TTL_SECS,
	GithubLinkPayload,
	GithubSetupPayload,
	create_session,
	github_client,
};
use crate::{prelude::*, redis::keys as redis_keys};

/// GitHub token exchange response
#[derive(Deserialize)]
struct GitHubTokenResponse {
	access_token: Option<String>,
}

/// GitHub user profile (`GET /user`)
#[derive(Deserialize)]
struct GitHubUserProfile {
	id: i64,
	login: String,
	name: Option<String>,
	email: Option<String>,
}

/// One entry from `GET /user/emails`
#[derive(Deserialize)]
struct GitHubEmail {
	email: String,
	primary: bool,
	verified: bool,
}

/// `POST /auth/social-login/github/callback`
///
/// Verifies the CSRF state, exchanges the code for a GitHub token, fetches the
/// user profile, and resolves which of the three paths to take.
pub async fn github_oauth_callback(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: GithubOAuthCallbackPath {},
				query: (),
				headers: GithubOAuthCallbackRequestHeaders { user_agent },
				// Rename `state` (OAuth CSRF token) to `csrf_state` to avoid
				// shadowing `state` (AppState) at the outer destructuring level.
				body: GithubOAuthCallbackRequestProcessed {
					code,
					state: csrf_state,
				},
			},
		database,
		redis,
		client_ip,
		mut state,
	}: AppRequest<'_, GithubOAuthCallbackRequest>,
) -> Result<AppResponse<GithubOAuthCallbackRequest>, ErrorType> {
	trace!("Processing GitHub OAuth callback");

	// Verify and atomically consume the CSRF state token.
	let state_key = redis_keys::social_login_state(&OAuthProvider::Github, &csrf_state);
	redis
		.getdel::<Option<String>>(&state_key)
		.await
		.inspect_err(|err| error!("Redis error consuming GitHub state: {err}"))?
		.ok_or(ErrorType::GithubOAuthFailed)?;

	// Exchange the authorization code for a GitHub access token.
	let client = github_client();

	let token_resp = client
		.post("https://github.com/login/oauth/access_token")
		.header("Accept", "application/json")
		.form(&[
			(
				"client_id",
				state.config.social_login.github.client_id.as_ref(),
			),
			(
				"client_secret",
				state.config.social_login.github.client_secret.as_ref(),
			),
			("code", code.as_ref()),
			(
				"redirect_uri",
				state.config.social_login.github.callback_url.as_ref(),
			),
		])
		.send()
		.await
		.inspect_err(|err| error!("Error exchanging GitHub code: {err}"))
		.map_err(|_| ErrorType::GithubOAuthFailed)?
		.json::<GitHubTokenResponse>()
		.await
		.inspect_err(|err| error!("Error parsing GitHub token response: {err}"))
		.map_err(|_| ErrorType::GithubOAuthFailed)?;

	let github_access_token = token_resp
		.access_token
		.ok_or(ErrorType::GithubOAuthFailed)?;

	// Fetch GitHub user profile.
	let github_user = client
		.get("https://api.github.com/user")
		.bearer_auth(&github_access_token)
		.send()
		.await
		.inspect_err(|err| error!("Error fetching GitHub user profile: {err}"))
		.map_err(|_| ErrorType::GithubOAuthFailed)?
		.json::<GitHubUserProfile>()
		.await
		.inspect_err(|err| error!("Error parsing GitHub user profile: {err}"))
		.map_err(|_| ErrorType::GithubOAuthFailed)?;

	// Fetch GitHub primary verified email.
	let github_emails = client
		.get("https://api.github.com/user/emails")
		.bearer_auth(&github_access_token)
		.send()
		.await
		.inspect_err(|err| error!("Error fetching GitHub emails: {err}"))
		.map_err(|_| ErrorType::GithubOAuthFailed)?
		.error_for_status()
		.inspect_err(|err| error!("GitHub emails endpoint returned error status: {err}"))
		.map_err(|_| ErrorType::GithubOAuthFailed)?
		.json::<Vec<GitHubEmail>>()
		.await
		.inspect_err(|err| error!("Error parsing GitHub emails response: {err}"))
		.map_err(|_| ErrorType::GithubOAuthFailed)?;

	// Only the *primary verified* email is trusted. `github_user.email` is the
	// user's public profile email, which may be unset or unverified — falling
	// back to it would let an attacker who controls an unverified address on
	// the victim's GitHub account match against an existing Patr recovery
	// email. Verified-primary is the only safe identifier.
	let github_email = github_emails
		.iter()
		.find(|e| e.primary && e.verified)
		.map(|e| e.email.to_lowercase());

	// Path A: existing GitHub link
	let github_external_id = github_user.id.to_string();
	if let Some(row) = query!(
		r#"
		SELECT user_id FROM user_social_login
		WHERE provider = 'github' AND external_id = $1;
		"#,
		github_external_id,
	)
	.fetch_optional(&mut **database)
	.await?
	{
		trace!(
			"Path A: existing GitHub link found for external_id={}",
			github_external_id
		);
		let (access_token, refresh_token) = create_session(
			database,
			&mut state,
			redis,
			client_ip,
			user_agent.to_string(),
			row.user_id.into(),
		)
		.await?;

		return AppResponse::builder()
			.body(GithubOAuthCallbackResponse {
				status: GithubCallbackStatus::LoggedIn {
					access_token,
					refresh_token,
				},
			})
			.headers(())
			.status_code(StatusCode::OK)
			.build()
			.into_result();
	}

	// Path A returned above without needing an email. Both LinkRequired and
	// SetupRequired need a verified address — anything else leaves the setup
	// endpoint with no recovery email and the linker matching against an
	// unverified identifier. Fail the callback cleanly.
	let Some(github_email) = github_email else {
		trace!("GitHub returned no primary verified email — failing the OAuth callback");
		return Err(ErrorType::GithubOAuthFailed);
	};

	// Path B: email matches existing Patr account
	if let Some(row) = query!(
		r#"
		SELECT "user".id AS "id!"
		FROM "user"
		WHERE recovery_email = $1
		UNION
		SELECT user_email.user_id AS "id!"
		FROM user_email
		WHERE email = $1
		LIMIT 1;
		"#,
		&github_email,
	)
	.fetch_optional(&mut **database)
	.await?
	{
		trace!("Path B: email match found for {}", github_email);

		let link_token = Uuid::new_v4().to_string();
		let payload = serde_json::to_string(&GithubLinkPayload {
			user_id: row.id.into(),
			external_id: github_external_id.clone(),
			email: Some(github_email.clone()),
		})
		.map_err(ErrorType::server_error)?;

		redis
			.setex(
				redis_keys::social_login_link(&OAuthProvider::Github, &link_token),
				GITHUB_LINK_TTL_SECS,
				payload,
			)
			.await
			.inspect_err(|err| error!("Redis error storing link token: {err}"))?;

		return AppResponse::builder()
			.body(GithubOAuthCallbackResponse {
				status: GithubCallbackStatus::LinkRequired { link_token },
			})
			.headers(())
			.status_code(StatusCode::OK)
			.build()
			.into_result();
	}

	// Path C: new user — direct to setup form
	trace!("Path C: new GitHub user, directing to setup page");

	let (prefilled_first_name, prefilled_last_name) =
		split_display_name(github_user.name.as_deref());

	let setup_token = Uuid::new_v4().to_string();
	let payload = serde_json::to_string(&GithubSetupPayload {
		external_id: github_external_id,
		email: Some(github_email.clone()),
	})
	.map_err(ErrorType::server_error)?;

	redis
		.setex(
			redis_keys::social_login_setup(&OAuthProvider::Github, &setup_token),
			GITHUB_SETUP_TTL_SECS,
			payload,
		)
		.await
		.inspect_err(|err| error!("Redis error storing setup token: {err}"))?;

	AppResponse::builder()
		.body(GithubOAuthCallbackResponse {
			status: GithubCallbackStatus::SetupRequired {
				setup_token,
				prefilled_username: github_user.login,
				prefilled_first_name,
				prefilled_last_name,
				prefilled_email: github_email,
			},
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

/// Splits a GitHub display name into `(first_name, last_name)`. Returns
/// `("", "")` when the name is absent or empty so the setup form forces the
/// user to fill the fields in.
fn split_display_name(name: Option<&str>) -> (String, String) {
	let Some(trimmed) = name.map(str::trim).filter(|s| !s.is_empty()) else {
		return (String::new(), String::new());
	};
	let mut parts = trimmed.splitn(2, ' ');
	let first = parts.next().unwrap_or("").to_string();
	let last = parts.next().unwrap_or("").trim().to_string();
	(first, last)
}
