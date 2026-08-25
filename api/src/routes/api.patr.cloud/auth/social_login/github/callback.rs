use std::{num::ParseFloatError, ops::Add};

use argon2::{Algorithm, PasswordHasher, Version, password_hash::generate_salt};
use axum::http::StatusCode;
use jsonwebtoken::EncodingKey;
use models::api::auth::*;
use rustis::commands::StringCommands;
use serde::Deserialize;
use sqlx::types::ipnetwork::IpNetwork;
use time::OffsetDateTime;

use crate::{
	models::{
		access_token_data::AccessTokenData,
		social_login::{GITHUB_CLIENT, GithubSetupPayload, GithubStatePayload},
	},
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
	login: String,
	name: Option<String>,
}

/// One entry from `GET /user/emails`
#[derive(Deserialize)]
struct GitHubEmail {
	email: String,
	primary: bool,
	verified: bool,
}

/// `POST /auth/social-login/{provider}/callback`
///
/// Verifies the CSRF state, exchanges the code for a provider access token,
/// fetches the user profile, and resolves which path to take.
pub async fn social_login_callback(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: SocialLoginCallbackPath { provider },
				query: (),
				headers: SocialLoginCallbackRequestHeaders { user_agent },
				// Rename `state` (OAuth CSRF token) to `csrf_state` to avoid
				// shadowing `state` (AppState) at the outer destructuring level.
				body: SocialLoginCallbackRequestProcessed {
					code,
					state: csrf_state,
				},
			},
		database,
		redis,
		client_ip,
		state,
	}: AppRequest<'_, SocialLoginCallbackRequest>,
) -> Result<AppResponse<SocialLoginCallbackRequest>, ErrorType> {
	trace!("Processing {provider} OAuth callback");

	#[expect(irrefutable_let_patterns)]
	let SocialLoginProvider::GitHub = provider else {
		return Err(ErrorType::SocialLoginFailed);
	};

	// Atomically consume the CSRF state token. Token must be the
	// `Anonymous` variant — an `Authenticated` token belongs to the
	// connect-flow callback and isn't valid here.
	let GithubStatePayload::Anonymous = serde_json::from_str::<GithubStatePayload>(
		&redis
			.getdel::<Option<String>>(redis::keys::social_login_state(&provider, &csrf_state))
			.await
			.inspect_err(|err| error!("Redis error consuming GitHub state: {err}"))?
			.ok_or(ErrorType::SocialLoginFailed)?,
	)
	.map_err(ErrorType::server_error)?
	else {
		warn!("GitHub state token used on the auth callback was not an Anonymous-variant token");
		return Err(ErrorType::SocialLoginFailed);
	};

	// Exchange the authorization code for a GitHub access token.
	let github_access_token = GITHUB_CLIENT
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
		.map_err(|_| ErrorType::SocialLoginFailed)?
		.json::<GitHubTokenResponse>()
		.await
		.inspect_err(|err| error!("Error parsing GitHub token response: {err}"))
		.map_err(|_| ErrorType::SocialLoginFailed)?
		.access_token
		.ok_or(ErrorType::SocialLoginFailed)?;

	// Fetch GitHub user profile.
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

	// Only the *primary verified* email is trusted. `github_user.email` is the
	// user's public profile email, which may be unset or unverified — falling
	// back to it would let an attacker who controls an unverified address on
	// the victim's GitHub account match against an existing Patr recovery
	// email. Verified-primary is the only safe identifier.
	let Some(github_email) = GITHUB_CLIENT
		.get("https://api.github.com/user/emails")
		.bearer_auth(&github_access_token)
		.send()
		.await
		.inspect_err(|err| error!("Error fetching GitHub emails: {err}"))
		.map_err(|_| ErrorType::SocialLoginFailed)?
		.error_for_status()
		.inspect_err(|err| error!("GitHub emails endpoint returned error status: {err}"))
		.map_err(|_| ErrorType::SocialLoginFailed)?
		.json::<Vec<GitHubEmail>>()
		.await
		.inspect_err(|err| error!("Error parsing GitHub emails response: {err}"))
		.map_err(|_| ErrorType::SocialLoginFailed)?
		.iter()
		.find(|e| e.primary && e.verified)
		.map(|e| e.email.trim().to_lowercase())
	else {
		trace!("GitHub returned no primary verified email — failing the OAuth callback");
		return Err(ErrorType::SocialLoginFailed);
	};

	let user_id = 'connected: {
		// Check if this GitHub account is already linked to a Patr account.
		let already_linked_user = query!(
			r#"
			SELECT
				user_id AS "user_id: Uuid"
			FROM
				user_social_login
			WHERE
				provider = 'github' AND
				external_id = $1;
			"#,
			github_user.id.to_string(),
		)
		.fetch_optional(&mut **database)
		.await?
		.map(|row| row.user_id);

		// Path A: already linked → log the user in
		if let Some(user_id) = already_linked_user {
			trace!("GitHub account already linked to a Patr account, logging in");
			break 'connected user_id;
		}

		// Path B: verified GitHub email matches an existing Patr account → bind
		// the GitHub identity inline and log the user in. No confirmation step:
		// anyone who controls the verified email already has account-takeover
		// power via password reset, so the extra click adds friction without
		// security.
		let connected_account = query!(
			r#"
			SELECT
				"user".id AS "id!: Uuid"
			FROM
				"user"
			WHERE
				"user".email = $1::CITEXT;
			"#,
			&github_email,
		)
		.fetch_optional(&mut **database)
		.await?
		.map(|row| row.id);

		if let Some(matched_user_id) = connected_account {
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
					)
				ON CONFLICT (provider, external_id)
				DO NOTHING;
				"#,
				matched_user_id as _,
				github_user.id.to_string(),
				OffsetDateTime::now_utc(),
			)
			.execute(&mut **database)
			.await?;

			trace!("GitHub email matches an existing Patr account, linking and logging in");
			break 'connected matched_user_id;
		}

		// No existing link, and the email doesn't match any existing account — new user
		trace!("New GitHub user, directing to setup page");

		// Pre-fill the setup form with the GitHub display name split on the first
		// space. Empty strings when GitHub returned no display name — the setup
		// form already requires both fields, so the user is forced to fill them.
		let (prefilled_first_name, prefilled_last_name) = github_user
			.name
			.as_deref()
			.map(str::trim)
			.filter(|s| !s.is_empty())
			.map(|n| n.split_once(' ').unwrap_or((n, "")))
			.map(|(first, last)| (first.to_string(), last.trim().to_string()))
			.unwrap_or_default();

		let setup_token = Uuid::new_v4().to_string();

		redis
			.setex(
				redis::keys::social_login_setup(&provider, &setup_token),
				600, // 10 mins
				serde_json::to_string(&GithubSetupPayload {
					external_id: github_user.id.to_string(),
					email: github_email.clone(),
				})?,
			)
			.await
			.inspect_err(|err| error!("Redis error storing setup token: {err}"))?;

		return AppResponse::builder()
			.body(SocialLoginCallbackResponse {
				status: GithubCallbackStatus::SetupRequired {
					setup_token,
					prefilled_first_name,
					prefilled_last_name,
					prefilled_email: github_email,
				},
			})
			.headers(())
			.status_code(StatusCode::OK)
			.build()
			.into_result();
	};

	// Either the user just linked their GitHub account or had previously linked it,
	// they are now authenticated and we can create a session for them.

	let now = OffsetDateTime::now_utc();

	let refresh_token = Uuid::new_v4().to_string();
	let hashed_refresh_token = argon2::Argon2::new_with_secret(
		state.config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.inspect_err(|err| error!("Error creating Argon2: {err}"))
	.map_err(ErrorType::server_error)?
	.hash_password_with_salt(refresh_token.as_bytes(), &generate_salt())
	.inspect_err(|err| error!("Error hashing refresh token: {err}"))
	.map_err(ErrorType::server_error)?
	.to_string();
	let refresh_token_expiry = now.add(constants::INACTIVE_REFRESH_TOKEN_VALIDITY);

	let ip_info = ip::lookup(client_ip, &state).await?;

	if !cfg!(debug_assertions) && ip_info.bogon.unwrap_or(false) {
		return Err(ErrorType::server_error(format!(
			"cannot use bogon IP address: `{}`",
			client_ip
		)));
	}

	let (lat, lng) = if cfg!(debug_assertions) {
		(0f64, 0f64)
	} else {
		ip_info
			.loc
			.split_once(',')
			.map(|(lat, lng)| {
				Ok::<_, ParseFloatError>((
					lat.parse::<f64>().inspect_err(|err| {
						info!("Error parsing latitude: `{lat}` - {err}");
					})?,
					lng.parse::<f64>().inspect_err(|err| {
						info!("Error parsing longitude: `{lng}` - {err}");
					})?,
				))
			})
			.ok_or_else(|| {
				ErrorType::server_error(format!("unknown latitude and longitude: {}", ip_info.loc))
			})??
	};

	let country = ip_info.country;
	let region = ip_info.region;
	let city = ip_info.city;
	let timezone = ip_info.timezone.unwrap_or_default();

	let user_agent = user_agent.to_string();

	let login_id = query!(
		r#"
		WITH client AS (
			INSERT INTO
				actor_client(id, actor_client_type)
			VALUES
				(GENERATE_LOGIN_ID(), 'user_login')
			RETURNING id
		)
		INSERT INTO
			user_login(
				login_id,
				user_id,
				login_type,
				created
			)
		SELECT
			client.id, $1, 'web_login', $2
		FROM
			client
		RETURNING user_login.login_id AS "login_id: Uuid";
		"#,
		user_id as _,
		now,
	)
	.fetch_one(&mut **database)
	.await?
	.login_id;

	query!(
		r#"
		INSERT INTO
			web_login(
				login_id,
				original_login_id,
				user_id,

				refresh_token,
				token_expiry,

				created,
				created_ip,
				created_location,
				created_user_agent,
				created_country,
				created_region,
				created_city,
				created_timezone
			)
		VALUES
			(
				$1,
				NULL,
				$2,

				$3,
				$4,

				$5,
				$6,
				ST_SetSRID(POINT($7, $8)::GEOMETRY, 4326),
				$9,
				$10,
				$11,
				$12,
				$13
			);
		"#,
		login_id as _,
		user_id as _,
		hashed_refresh_token,
		refresh_token_expiry,
		now,
		IpNetwork::from(client_ip),
		lat,
		lng,
		user_agent,
		country,
		region,
		city,
		timezone,
	)
	.execute(&mut **database)
	.await?;

	let access_token = AccessTokenData {
		iss: constants::JWT_ISSUER.to_string(),
		sub: login_id,
		aud: OneOrMore::One(constants::PATR_JWT_AUDIENCE.to_string()),
		exp: now.add(constants::ACCESS_TOKEN_VALIDITY),
		nbf: now,
		iat: now,
		jti: Uuid::now_v1(),
	};

	let access_token = jsonwebtoken::encode(
		&Default::default(),
		&access_token,
		&EncodingKey::from_secret(state.config.jwt_secret.as_ref()),
	)
	.inspect_err(|err| error!("Error encoding JWT: {err}"))?;

	let refresh_token = format!("{login_id}.{refresh_token}");

	AppResponse::builder()
		.body(SocialLoginCallbackResponse {
			status: GithubCallbackStatus::LoggedIn {
				access_token,
				refresh_token,
			},
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
