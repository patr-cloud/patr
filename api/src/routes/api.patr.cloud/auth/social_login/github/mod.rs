use std::{ops::Add, sync::OnceLock};

use argon2::{Algorithm, PasswordHasher, Version, password_hash::generate_salt};
use jsonwebtoken::EncodingKey;
use serde::{Deserialize, Serialize};
use sqlx::types::ipnetwork::IpNetwork;
use time::OffsetDateTime;

use crate::{models::access_token_data::AccessTokenData, prelude::*};

/// Handler for `POST /auth/social-login/github/callback` — completes the OAuth
/// flow
mod callback;
/// Handler for `GET /auth/social-login/github` — initiates the OAuth flow
mod initiate;
/// Handler for `POST /auth/social-login/github/link` — links a GitHub identity
/// to an existing Patr account
mod link;
/// Handler for `POST /auth/social-login/github/setup` — creates a new Patr
/// account from a GitHub identity
mod setup;

pub use self::{callback::*, initiate::*, link::*, setup::*};

/// Stored in Redis for the link flow
#[derive(Serialize, Deserialize)]
pub(super) struct GithubLinkPayload {
	pub user_id: Uuid,
	pub external_id: String,
	pub email: Option<String>,
}

/// Stored in Redis for the setup flow
#[derive(Serialize, Deserialize)]
pub(super) struct GithubSetupPayload {
	pub external_id: String,
	pub email: Option<String>,
}

/// Link-confirmation token validity: 5 minutes
pub(super) const GITHUB_LINK_TTL_SECS: u64 = 300;
/// Setup token validity: 10 minutes
pub(super) const GITHUB_SETUP_TTL_SECS: u64 = 600;

/// Lazily initialised HTTP client for reuse across GitHub API calls. GitHub
/// requires a `User-Agent` header on every request.
static GITHUB_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Returns the shared GitHub HTTP client, initialising it on first use.
pub(super) fn github_client() -> &'static reqwest::Client {
	GITHUB_CLIENT.get_or_init(|| {
		reqwest::Client::builder()
			.user_agent("patr-api/1.0")
			.build()
			.expect("failed to build GitHub HTTP client")
	})
}

/// Creates a new `user_login` + `web_login` session and returns
/// `(access_token_jwt, formatted_refresh_token)`.
pub(super) async fn create_session(
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
