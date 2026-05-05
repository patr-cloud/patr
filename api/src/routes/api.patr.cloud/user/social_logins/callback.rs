use axum::http::StatusCode;
use models::api::{auth::SocialLoginProvider, user::*};
use rustis::commands::StringCommands;
use serde::Deserialize;
use time::OffsetDateTime;

use crate::{
	models::social_login::{GITHUB_CLIENT, GithubStatePayload},
	prelude::*,
};

/// GitHub token exchange response
#[derive(Deserialize)]
struct GitHubTokenResponse {
	access_token: Option<String>,
}

/// GitHub user profile (`GET /user`)
#[derive(Deserialize)]
struct GitHubUserProfile {
	id: i64,
}

pub async fn social_login_callback(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ConnectSocialLoginCallbackPath { provider },
				query: (),
				headers:
					ConnectSocialLoginCallbackRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body:
					ConnectSocialLoginCallbackRequestProcessed {
						code,
						state: csrf_state,
					},
			},
		database,
		redis,
		user_data,
		state,
		..
	}: AuthenticatedAppRequest<'_, ConnectSocialLoginCallbackRequest>,
) -> Result<AppResponse<ConnectSocialLoginCallbackRequest>, ErrorType> {
	trace!(
		"Processing GitHub connect callback for user {}",
		user_data.id
	);

	#[expect(irrefutable_let_patterns)]
	let SocialLoginProvider::GitHub = provider else {
		return Err(ErrorType::SocialLoginFailed);
	};

	// Atomically consume the state token and recover the `user_id` recorded
	// at initiate time. Token must be of the `Connect` variant — an `Auth`
	// token belongs to the unauthenticated sign-in callback and isn't valid
	// here.
	let payload = serde_json::from_str::<GithubStatePayload>(
		&redis
			.getdel::<Option<String>>(redis::keys::social_login_state(&provider, &csrf_state))
			.await
			.inspect_err(|err| error!("Redis error consuming GitHub connect state: {err}"))?
			.ok_or(ErrorType::SocialLoginFailed)?,
	)
	.map_err(ErrorType::server_error)?;

	let GithubStatePayload::Authenticated {
		user_id: state_user_id,
	} = payload
	else {
		warn!("GitHub state token used on the connect callback was not a Connect-variant token");
		return Err(ErrorType::SocialLoginFailed);
	};

	// The state token's user_id must match the caller's JWT. Guards against
	// a connect started in tab A (logged in as Alice) accidentally
	// completing in tab B after a re-login as Bob.
	if state_user_id != user_data.id {
		warn!(
			"GitHub connect-state user_id ({}) does not match caller ({})",
			state_user_id, user_data.id
		);
		return Err(ErrorType::SocialLoginFailed);
	}

	// Exchange the authorization code for a GitHub access token.
	let token_resp = GITHUB_CLIENT
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
				state
					.config
					.social_login
					.github
					.connect_callback_url
					.as_ref(),
			),
		])
		.send()
		.await
		.inspect_err(|err| error!("Error exchanging GitHub code: {err}"))
		.map_err(|_| ErrorType::SocialLoginFailed)?
		.json::<GitHubTokenResponse>()
		.await
		.inspect_err(|err| error!("Error parsing GitHub token response: {err}"))
		.map_err(|_| ErrorType::SocialLoginFailed)?;

	let github_access_token = token_resp
		.access_token
		.ok_or(ErrorType::SocialLoginFailed)?;

	// Fetch GitHub user profile — only the numeric `id` is needed; the
	// connect flow doesn't touch email or display name.
	let github_user = GITHUB_CLIENT
		.get("https://api.github.com/user")
		.bearer_auth(&github_access_token)
		.send()
		.await
		.inspect_err(|err| error!("Error fetching GitHub user profile: {err}"))
		.map_err(|_| ErrorType::SocialLoginFailed)?
		.json::<GitHubUserProfile>()
		.await
		.inspect_err(|err| error!("Error parsing GitHub user profile: {err}"))
		.map_err(|_| ErrorType::SocialLoginFailed)?;

	let github_external_id = github_user.id.to_string();

	// Idempotent insert. `ON CONFLICT (provider, external_id)` covers both
	//   (a) the same user re-running connect — UNIQUE(user_id, provider)
	//       conflicts, no-op.
	//   (b) the GitHub identity already linked to *another* Patr account —
	//       PRIMARY KEY (provider, external_id) conflicts, no-op.
	// In case (b) we want to fail visibly so the user knows their GitHub
	// account is attached elsewhere — return ResourceAlreadyExists when the
	// existing row's user_id isn't theirs.
	let existing_owner = query!(
		r#"
		SELECT
			user_id
		FROM
			user_social_login
		WHERE
			provider = 'github' AND
			external_id = $1;
		"#,
		github_external_id,
	)
	.fetch_optional(&mut **database)
	.await?
	.map(|row| Uuid::from(row.user_id));

	match existing_owner {
		Some(owner) if owner != user_data.id => {
			warn!(
				"GitHub identity {} is already linked to user {} (caller: {})",
				github_external_id, owner, user_data.id
			);
			return Err(ErrorType::ResourceAlreadyExists);
		}
		Some(_) => {
			// Already linked to this user — nothing to do.
		}
		None => {
			query!(
				r#"
				INSERT INTO
					user_social_login(
						user_id,
						provider,
						external_id,
						linked_at
					)
				VALUES
					(
						$1,
						'github',
						$2,
						$3
					);
				"#,
				user_data.id as _,
				github_external_id,
				OffsetDateTime::now_utc(),
			)
			.execute(&mut **database)
			.await?;
		}
	}

	AppResponse::builder()
		.body(ConnectSocialLoginCallbackResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
