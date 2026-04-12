use std::ops::Add;

use argon2::{Algorithm, PasswordHasher, Version, password_hash::generate_salt};
use axum::http::StatusCode;
use jsonwebtoken::EncodingKey;
use models::api::auth::*;
use rustis::commands::{GenericCommands, StringCommands};
use serde::{Deserialize, Serialize};
use sqlx::types::ipnetwork::IpNetwork;
use time::OffsetDateTime;

use crate::{models::access_token_data::AccessTokenData, prelude::*, redis::keys as redis_keys};

// ─── GitHub API response structs ─────────────────────────────────────────────

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

/// Stored in Redis for the link flow
#[derive(Serialize, Deserialize)]
struct GithubLinkPayload {
	user_id: Uuid,
	external_id: String,
	email: Option<String>,
}

/// Stored in Redis for the setup flow
#[derive(Serialize, Deserialize)]
struct GithubSetupPayload {
	external_id: String,
	email: Option<String>,
}

// ─── TTLs ────────────────────────────────────────────────────────────────────

/// CSRF state token validity: 10 minutes
const GITHUB_STATE_TTL_SECS: u64 = 600;
/// Link-confirmation token validity: 5 minutes
const GITHUB_LINK_TTL_SECS: u64 = 300;
/// Setup token validity: 10 minutes
const GITHUB_SETUP_TTL_SECS: u64 = 600;

// ─── GitHub HTTP client
// ───────────────────────────────────────────────────────

fn github_client() -> reqwest::Client {
	reqwest::Client::builder()
		.user_agent("patr-api/1.0")
		.build()
		.expect("failed to build GitHub HTTP client")
}

// ─── Handler: initiate ───────────────────────────────────────────────────────

/// `GET /auth/github`
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
			redis_keys::github_oauth_state(&oauth_state_token),
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
		.append_pair("client_id", &state.config.github_oauth.client_id)
		.append_pair("redirect_uri", &state.config.github_oauth.callback_url)
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

// ─── Handler: callback ───────────────────────────────────────────────────────

/// `POST /auth/github/callback`
///
/// Verifies the CSRF state, exchanges the code for a GitHub token, fetches the
/// user profile, and resolves which of the three paths to take.
pub async fn github_oauth_callback(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: GithubOAuthCallbackPath,
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

	// ── Step 1: Verify CSRF state ────────────────────────────────────────────
	let state_key = redis_keys::github_oauth_state(&csrf_state);
	let state_val = redis
		.get::<Option<String>>(&state_key)
		.await
		.inspect_err(|err| error!("Redis error checking GitHub state: {err}"))?;

	if state_val.is_none() {
		return Err(ErrorType::GithubOAuthFailed);
	}

	// Consume the state (one-time use)
	redis
		.del(&state_key)
		.await
		.inspect_err(|err| error!("Redis error deleting GitHub state: {err}"))?;

	// ── Step 2: Exchange code for GitHub access token ────────────────────────
	let client = github_client();

	let token_resp = client
		.post("https://github.com/login/oauth/access_token")
		.header("Accept", "application/json")
		.form(&[
			("client_id", state.config.github_oauth.client_id.as_ref()),
			(
				"client_secret",
				state.config.github_oauth.client_secret.as_ref(),
			),
			("code", code.as_ref()),
			(
				"redirect_uri",
				state.config.github_oauth.callback_url.as_ref(),
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

	// ── Step 3: Fetch GitHub user profile ───────────────────────────────────
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

	// ── Step 4: Fetch GitHub primary verified email ──────────────────────────
	let github_emails = client
		.get("https://api.github.com/user/emails")
		.bearer_auth(&github_access_token)
		.send()
		.await
		.inspect_err(|err| error!("Error fetching GitHub emails: {err}"))
		.map_err(|_| ErrorType::GithubOAuthFailed)?
		.json::<Vec<GitHubEmail>>()
		.await
		.unwrap_or_default();

	let primary_verified_email = github_emails
		.iter()
		.find(|e| e.primary && e.verified)
		.map(|e| e.email.to_lowercase());

	let github_email = github_user
		.email
		.as_deref()
		.map(str::to_lowercase)
		.or(primary_verified_email);

	// ── Step 5: Account resolution ───────────────────────────────────────────

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
				status: GithubCallbackStatus::LoggedIn,
				access_token: Some(access_token),
				refresh_token: Some(refresh_token),
				link_token: None,
				setup_token: None,
				prefilled_username: None,
				prefilled_first_name: None,
				prefilled_last_name: None,
				prefilled_email: None,
			})
			.headers(())
			.status_code(StatusCode::OK)
			.build()
			.into_result();
	}

	// Path B: email matches existing Patr account
	if let Some(ref email) = github_email {
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
			email,
		)
		.fetch_optional(&mut **database)
		.await?
		{
			trace!("Path B: email match found for {}", email);

			let link_token = Uuid::new_v4().to_string();
			let payload = serde_json::to_string(&GithubLinkPayload {
				user_id: row.id.into(),
				external_id: github_external_id.clone(),
				email: github_email.clone(),
			})
			.map_err(ErrorType::server_error)?;

			redis
				.setex(
					redis_keys::github_oauth_link(&link_token),
					GITHUB_LINK_TTL_SECS,
					payload,
				)
				.await
				.inspect_err(|err| error!("Redis error storing link token: {err}"))?;

			return AppResponse::builder()
				.body(GithubOAuthCallbackResponse {
					status: GithubCallbackStatus::LinkRequired,
					access_token: None,
					refresh_token: None,
					link_token: Some(link_token),
					setup_token: None,
					prefilled_username: None,
					prefilled_first_name: None,
					prefilled_last_name: None,
					prefilled_email: None,
				})
				.headers(())
				.status_code(StatusCode::OK)
				.build()
				.into_result();
		}
	}

	// Path C: new user — direct to setup form
	trace!("Path C: new GitHub user, directing to setup page");

	let (prefilled_first_name, prefilled_last_name) =
		split_display_name(github_user.name.as_deref());

	let setup_token = Uuid::new_v4().to_string();
	let payload = serde_json::to_string(&GithubSetupPayload {
		external_id: github_external_id,
		email: github_email.clone(),
	})
	.map_err(ErrorType::server_error)?;

	redis
		.setex(
			redis_keys::github_oauth_setup(&setup_token),
			GITHUB_SETUP_TTL_SECS,
			payload,
		)
		.await
		.inspect_err(|err| error!("Redis error storing setup token: {err}"))?;

	AppResponse::builder()
		.body(GithubOAuthCallbackResponse {
			status: GithubCallbackStatus::SetupRequired,
			access_token: None,
			refresh_token: None,
			link_token: None,
			setup_token: Some(setup_token),
			prefilled_username: Some(github_user.login),
			prefilled_first_name: Some(prefilled_first_name),
			prefilled_last_name: Some(prefilled_last_name),
			prefilled_email: github_email,
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

// ─── Handler: link ───────────────────────────────────────────────────────────

/// `POST /auth/github/link`
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

	// Fetch and consume the link token
	let link_key = redis_keys::github_oauth_link(&link_token);
	let raw = redis
		.get::<Option<String>>(&link_key)
		.await
		.inspect_err(|err| error!("Redis error fetching link token: {err}"))?
		.ok_or(ErrorType::GithubOAuthFailed)?;

	redis
		.del(&link_key)
		.await
		.inspect_err(|err| error!("Redis error deleting link token: {err}"))?;

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

// ─── Handler: setup ──────────────────────────────────────────────────────────

/// `POST /auth/github/setup`
///
/// Creates a new Patr account from a confirmed GitHub identity after the user
/// has reviewed/edited the pre-filled profile details.
pub async fn github_oauth_setup(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: GithubOAuthSetupPath,
				query: (),
				headers: GithubOAuthSetupRequestHeaders { user_agent },
				body:
					GithubOAuthSetupRequestProcessed {
						setup_token,
						username,
						first_name,
						last_name,
					},
			},
		database,
		redis,
		client_ip,
		mut state,
	}: AppRequest<'_, GithubOAuthSetupRequest>,
) -> Result<AppResponse<GithubOAuthSetupRequest>, ErrorType> {
	trace!("Processing GitHub OAuth account setup");

	// Fetch and consume the setup token
	let setup_key = redis_keys::github_oauth_setup(&setup_token);
	let raw = redis
		.get::<Option<String>>(&setup_key)
		.await
		.inspect_err(|err| error!("Redis error fetching setup token: {err}"))?
		.ok_or(ErrorType::GithubOAuthFailed)?;

	redis
		.del(&setup_key)
		.await
		.inspect_err(|err| error!("Redis error deleting setup token: {err}"))?;

	let payload: GithubSetupPayload =
		serde_json::from_str(&raw).map_err(ErrorType::server_error)?;

	// Require a verified email from GitHub — accounts with no verified email cannot
	// complete setup
	let recovery_email = payload.email.ok_or(ErrorType::GithubOAuthFailed)?;

	// Check username availability
	let username_taken = query!(r#"SELECT id FROM "user" WHERE username = $1;"#, &username,)
		.fetch_optional(&mut **database)
		.await?
		.is_some();

	if username_taken {
		return Err(ErrorType::UsernameUnavailable);
	}

	// Check email availability
	let email_taken = query!(
		r#"
		SELECT id AS "id!" FROM "user" WHERE recovery_email = $1
		UNION
		SELECT user_id AS "id!" FROM user_email WHERE email = $1
		LIMIT 1;
		"#,
		&recovery_email,
	)
	.fetch_optional(&mut **database)
	.await?
	.is_some();

	if email_taken {
		return Err(ErrorType::EmailUnavailable);
	}

	let now = OffsetDateTime::now_utc();
	let user_id = Uuid::new_v4();

	// Hash a random password (user authenticates via GitHub; can reset via email)
	let random_password = Uuid::new_v4().to_string();
	let hashed_password = argon2::Argon2::new_with_secret(
		state.config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.inspect_err(|err| error!("Error creating Argon2: {err}"))
	.map_err(ErrorType::server_error)?
	.hash_password_with_salt(random_password.as_bytes(), &generate_salt())
	.inspect_err(|err| error!("Error hashing password: {err}"))
	.map_err(ErrorType::server_error)?
	.to_string();

	// Defer constraints to allow the circular insert order
	query!(r#"SET CONSTRAINTS ALL DEFERRED;"#)
		.execute(&mut **database)
		.await?;

	query!(
		r#"
		INSERT INTO "user"(
			id, username, password,
			first_name, last_name, created,
			recovery_email, recovery_phone_country_code, recovery_phone_number,
			workspace_limit,
			password_reset_token, password_reset_token_expiry, password_reset_attempts,
			mfa_secret
		) VALUES (
			$1, $2, $3,
			$4, $5, $6,
			$7, NULL, NULL,
			$8,
			NULL, NULL, NULL,
			NULL
		);
		"#,
		user_id as _,
		&username,
		hashed_password,
		&first_name,
		&last_name,
		now,
		&recovery_email,
		constants::DEFAULT_WORKSPACE_LIMIT,
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"INSERT INTO user_email(user_id, email) VALUES ($1, $2);"#,
		user_id as _,
		&recovery_email,
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		INSERT INTO user_social_login(user_id, provider, external_id, linked_at)
		VALUES ($1, 'github', $2, $3);
		"#,
		user_id as _,
		payload.external_id,
		now,
	)
	.execute(&mut **database)
	.await?;

	query!(r#"SET CONSTRAINTS ALL IMMEDIATE;"#)
		.execute(&mut **database)
		.await?;

	let (access_token, refresh_token) = create_session(
		database,
		&mut state,
		redis,
		client_ip,
		user_agent.to_string(),
		user_id,
	)
	.await?;

	AppResponse::builder()
		.body(GithubOAuthSetupResponse {
			access_token,
			refresh_token,
		})
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}

// ─── Shared helpers
// ───────────────────────────────────────────────────────────

/// Creates a new `user_login` + `web_login` session and returns
/// `(access_token_jwt, formatted_refresh_token)`.
async fn create_session(
	database: &mut DatabaseTransaction,
	state: &mut AppState,
	redis: &mut rustis::client::Client,
	client_ip: std::net::IpAddr,
	user_agent: String,
	user_id: Uuid,
) -> Result<(String, String), ErrorType> {
	use std::num::ParseFloatError;

	let now = OffsetDateTime::now_utc();

	let refresh_token_raw = Uuid::new_v4().to_string();
	let hashed_refresh_token = argon2::Argon2::new_with_secret(
		state.config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.inspect_err(|err| error!("Error creating Argon2: {err}"))
	.map_err(ErrorType::server_error)?
	.hash_password_with_salt(refresh_token_raw.as_bytes(), &generate_salt())
	.inspect_err(|err| error!("Error hashing refresh token: {err}"))
	.map_err(ErrorType::server_error)?
	.to_string();

	let refresh_token_expiry = now.add(constants::INACTIVE_REFRESH_TOKEN_VALIDITY);

	let ip_info = ip::lookup(client_ip, redis, &state.config.ipinfo).await?;

	if !cfg!(debug_assertions) && ip_info.bogon.unwrap_or(false) {
		return Err(ErrorType::server_error(format!(
			"cannot use bogon IP address: `{}`",
			client_ip
		)));
	}

	let client_ip_network = IpNetwork::from(client_ip);

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

	let login_id = query!(
		r#"
		INSERT INTO user_login(login_id, user_id, login_type, created)
		VALUES (GENERATE_LOGIN_ID(), $1, 'web_login', $2)
		RETURNING login_id;
		"#,
		user_id as _,
		now,
	)
	.fetch_one(&mut **database)
	.await?
	.login_id
	.into();

	query!(
		r#"
		INSERT INTO web_login(
			login_id, original_login_id, user_id,
			refresh_token, token_expiry,
			created, created_ip, created_location, created_user_agent,
			created_country, created_region, created_city, created_timezone
		) VALUES (
			$1, NULL, $2,
			$3, $4,
			$5, $6, ST_SetSRID(POINT($7, $8)::GEOMETRY, 4326), $9,
			$10, $11, $12, $13
		);
		"#,
		login_id as _,
		user_id as _,
		hashed_refresh_token,
		refresh_token_expiry,
		now,
		client_ip_network,
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

	let access_token_jwt = jsonwebtoken::encode(
		&Default::default(),
		&access_token,
		&EncodingKey::from_secret(state.config.jwt_secret.as_ref()),
	)
	.inspect_err(|err| error!("Error encoding JWT: {err}"))?;

	let formatted_refresh_token = format!("{login_id}.{refresh_token_raw}");

	Ok((access_token_jwt, formatted_refresh_token))
}

/// Splits a GitHub display name into `(first_name, last_name)`.
/// Falls back to `("GitHub", "User")` when the name is absent or empty.
fn split_display_name(name: Option<&str>) -> (String, String) {
	match name.map(str::trim).filter(|s| !s.is_empty()) {
		None => ("GitHub".to_string(), "User".to_string()),
		Some(n) => {
			let mut parts = n.splitn(2, ' ');
			let first = parts.next().unwrap_or("GitHub").to_string();
			let last = parts
				.next()
				.filter(|s| !s.is_empty())
				.unwrap_or("User")
				.to_string();
			(first, last)
		}
	}
}
