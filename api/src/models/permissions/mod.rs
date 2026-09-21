use std::{collections::BTreeMap, net::IpAddr, ops::Sub, str::FromStr as _};

use argon2::{Algorithm, Argon2, PasswordHash, PasswordVerifier as _, Version};
use jsonwebtoken::{DecodingKey, TokenData, Validation};
use models::{ActorData, RequestActorData, UserLoginType};
use rustis::client::Client as RedisClient;
use time::OffsetDateTime;
use tokio::sync::OnceCell;

use crate::{
	models::{access_token_data::AccessTokenData, redis::ActorAuthDataCacheKind},
	prelude::*,
	utils::config::AppConfig,
};

/// A global map of Permission -> PermissionID for all permissions.
/// This is used to cache the permission IDs for faster lookup instead of
/// fetching it from the database every time.
#[doc(hidden)]
static PERMISSION_TO_ID_MAP: OnceCell<BTreeMap<Permission, Uuid>> = OnceCell::const_new();

/// Looks up the UUID for a given [`Permission`] in the database, caching
/// the full permission table on first call via [`PERMISSION_TO_ID_MAP`].
pub async fn get_permission_id(database: &mut DatabaseConnection, permission: Permission) -> Uuid {
	PERMISSION_TO_ID_MAP
		.get_or_init(async || {
			query!(
				r#"
				SELECT
					id AS "id: Uuid",
					name
				FROM
					permission;
				"#
			)
			.fetch_all(&mut *database)
			.await
			.expect("Failed to fetch permissions from the database")
			.into_iter()
			.map(|row| {
				(
					Permission::from_str(&row.name).expect("Invalid permission name"),
					row.id,
				)
			})
			.collect()
		})
		.await
		.get(&permission)
		.copied()
		.unwrap_or_else(|| {
			panic!("Permission {permission} does not exist in the database");
		})
}

/// Loads the auth data of a user API token.
mod api_token;
/// Loads the auth data of a service account.
mod service_account;
/// Loads the auth data of a web dashboard session.
mod web_dashboard;

/// The Redis cache of authenticated actors, and what marks it stale.
mod cache;

pub use self::cache::{mark_actor_stale, mark_all_stale, mark_login_stale, mark_workspace_stale};

/// Authenticate a bearer token and return who is making the request.
///
/// A token is either a JWT ([`authenticate_jwt`]) or a
/// `patrv1.{secret}.{login_id}` token ([`authenticate_opaque_token`]). Both
/// key the cache by login ID, so a hit costs no database round trip; a miss
/// looks the login up, verifies it, loads its permissions and caches the lot
/// until the token expires or something marks it stale (see [`cache`]).
///
/// Which kinds of client a route accepts is the caller's check.
pub async fn authenticate(
	database: &mut DatabaseConnection,
	redis: &mut RedisClient,
	config: &AppConfig,
	client_ip: IpAddr,
	token: &str,
) -> Result<RequestActorData, ErrorType> {
	match jsonwebtoken::decode::<AccessTokenData>(
		token,
		&DecodingKey::from_secret(config.jwt_secret.as_ref()),
		&{
			let mut validation = Validation::default();

			// We'll manually do this
			validation.validate_exp = false;
			validation.validate_nbf = false;
			validation.validate_aud = false;

			validation
		},
	) {
		Ok(TokenData { header: _, claims }) => authenticate_jwt(database, redis, claims).await,
		Err(err) => {
			trace!("Authentication header is not a JWT: {}", err);
			authenticate_opaque_token(database, redis, config, client_ip, token).await
		}
	}
}

/// Authenticate a JWT. Today every JWT is a web dashboard session; an OAuth
/// application's token will carry a claim saying so and branch here.
async fn authenticate_jwt(
	database: &mut DatabaseConnection,
	redis: &mut RedisClient,
	claims: AccessTokenData,
) -> Result<RequestActorData, ErrorType> {
	// The claims are checked on every request, cache hit or not: a login's
	// cache entry outlives any one of its access tokens.
	if claims.iss != constants::JWT_ISSUER {
		warn!("Invalid JWT issuer: {}", claims.iss);
		return Err(ErrorType::MalformedAccessToken);
	}
	trace!("JWT issuer valid");

	// The token should have been issued within the last `REFRESH_TOKEN_VALIDITY`
	// duration
	if OffsetDateTime::now_utc().sub(
		claims
			.jti
			.get_timestamp()
			.ok_or(ErrorType::MalformedAccessToken)?,
	) > AccessTokenData::REFRESH_TOKEN_VALIDITY
	{
		warn!("JWT is too old");
		return Err(ErrorType::AuthorizationTokenInvalid);
	}
	trace!("JWT JTI valid");

	if OffsetDateTime::now_utc() < claims.nbf {
		warn!("JWT is not valid yet");
		return Err(ErrorType::AuthorizationTokenInvalid);
	}
	trace!("JWT NBF valid");

	if OffsetDateTime::now_utc() > claims.exp {
		warn!("JWT has expired");
		return Err(ErrorType::AuthorizationTokenInvalid);
	}
	trace!("JWT EXP valid");

	if !claims
		.aud
		.clone()
		.into_iter()
		.any(|item| item == constants::PATR_JWT_AUDIENCE)
	{
		warn!(
			"Invalid JWT audience: `{}`",
			match &claims.aud {
				OneOrMore::One(aud) => aud.clone(),
				OneOrMore::Multiple(aud) => format!("[{}]", aud.join(", ")),
			}
		);
		return Err(ErrorType::MalformedAccessToken);
	}
	trace!("JWT audience valid");

	let entry = if let Some(entry) = cache::read(redis, &claims.sub).await {
		trace!("Cached auth data found for login `{}`", claims.sub);
		entry
	} else {
		let entry = web_dashboard::load_actor_auth_data(&mut *database, &claims.sub).await?;

		cache::write(
			redis,
			&claims.sub,
			&entry,
			constants::CACHED_PERMISSIONS_VALIDITY,
		)
		.await;

		entry
	};

	let ActorAuthDataCacheKind::WebLogin {
		email,
		first_name,
		last_name,
		created,
	} = entry.kind
	else {
		warn!("JWT presented for a login that is not a web login");
		return Err(ErrorType::AuthorizationTokenInvalid);
	};

	Ok(RequestActorData::builder()
		.id(entry.actor_id)
		.actor(ActorData::User {
			email,
			first_name,
			last_name,
			login: UserLoginType::WebLogin,
		})
		.created(created)
		.login_id(claims.sub)
		.permissions(entry.permissions)
		.build())
}

/// Authenticate a `patrv1.{secret}.{login_id}` token: a user API token or a
/// service account token. `actor_client` says which, and that kind's module
/// takes it from there.
async fn authenticate_opaque_token(
	database: &mut DatabaseConnection,
	redis: &mut RedisClient,
	config: &AppConfig,
	client_ip: IpAddr,
	token: &str,
) -> Result<RequestActorData, ErrorType> {
	trace!("Parsing authentication header as an API token");

	let Some(token) = token.strip_prefix("patrv1.") else {
		warn!("Authentication header is neither a JWT nor an API token");
		return Err(ErrorType::MalformedApiToken);
	};

	let (secret, login_id) = token.split_once('.').ok_or_else(|| {
		warn!("Invalid API token: missing secret/login-id separator");
		ErrorType::MalformedApiToken
	})?;

	let secret = Uuid::parse_str(secret).map_err(|err| {
		warn!("Invalid API token: secret is not a valid UUID: {}", err);
		ErrorType::MalformedApiToken
	})?;

	let login_id = Uuid::parse_str(login_id).map_err(|err| {
		warn!("Invalid API token: login ID is not a valid UUID: {}", err);
		ErrorType::MalformedApiToken
	})?;

	let entry = if let Some(entry) = cache::read(redis, &login_id).await {
		trace!("Cached auth data found for login `{login_id}`");
		entry
	} else {
		let Some(client) = query!(
			r#"
			SELECT
				actor_client_type::TEXT AS "actor_client_type!"
			FROM
				actor_client
			WHERE
				id = $1;
			"#,
			login_id as _,
		)
		.fetch_optional(&mut *database)
		.await?
		else {
			warn!("No login found for the API token");
			// No specific error for the login not being found, since we don't
			// want to leak information about whether a loginId is valid or if
			// it's expired
			return Err(ErrorType::AuthorizationTokenInvalid);
		};

		let (entry, ttl) = match client.actor_client_type.as_str() {
			"user_login" => api_token::load_actor_auth_data(&mut *database, &login_id).await?,
			"service_account" => {
				service_account::load_actor_auth_data(&mut *database, &login_id).await?
			}
			other => {
				error!("Unknown actor client type `{other}` for login `{login_id}`");
				return Err(ErrorType::server_error("unknown actor client type"));
			}
		};

		cache::write(redis, &login_id, &entry, ttl).await;
		entry
	};

	// Checked on every request, cache hit or not: the presented secret, and
	// for API tokens the client's IP.
	let (actor, created) = match entry.kind {
		ActorAuthDataCacheKind::ApiToken {
			email,
			first_name,
			last_name,
			created,
			allowed_ips,
			token_hash,
		} => {
			// Verify that the token presented is valid
			let Ok(password_hash) = PasswordHash::new(&token_hash) else {
				error!("Unable to parse password hash: {}", token_hash);
				return Err(ErrorType::server_error("password hash parsing failed"));
			};

			let success = Argon2::new_with_secret(
				config.password_pepper.as_bytes(),
				Algorithm::Argon2id,
				Version::V0x13,
				constants::HASHING_PARAMS,
			)
			.map_err(ErrorType::server_error)?
			.verify_password(secret.as_bytes(), &password_hash)
			.is_ok();

			if !success {
				warn!("API token has an invalid secret");
				return Err(ErrorType::AuthorizationTokenInvalid);
			}
			trace!("API token secret valid");

			if let Some(allowed_ips) = allowed_ips &&
				!allowed_ips
					.iter()
					.any(|ip_network| ip_network.contains(client_ip))
			{
				info!("API token not accessed from an allowed IP Address");
				return Err(ErrorType::DisallowedIpAddressForApiToken);
			}

			(
				ActorData::User {
					email,
					first_name,
					last_name,
					login: UserLoginType::ApiToken,
				},
				created,
			)
		}
		ActorAuthDataCacheKind::ServiceAccount {
			name,
			created,
			token_hash,
		} => {
			// Verify that the token presented is valid
			let Ok(password_hash) = PasswordHash::new(&token_hash) else {
				error!("Unable to parse password hash: {}", token_hash);
				return Err(ErrorType::server_error("password hash parsing failed"));
			};

			let success = Argon2::new_with_secret(
				config.password_pepper.as_bytes(),
				Algorithm::Argon2id,
				Version::V0x13,
				constants::HASHING_PARAMS,
			)
			.map_err(ErrorType::server_error)?
			.verify_password(secret.as_bytes(), &password_hash)
			.is_ok();

			if !success {
				warn!("API token has an invalid secret");
				return Err(ErrorType::AuthorizationTokenInvalid);
			}
			trace!("API token secret valid");

			(ActorData::ServiceAccount { name }, created)
		}
		ActorAuthDataCacheKind::WebLogin { .. } => {
			warn!("A web login's ID was presented as an API token");
			return Err(ErrorType::AuthorizationTokenInvalid);
		}
	};

	Ok(RequestActorData::builder()
		.id(entry.actor_id)
		.actor(actor)
		.created(created)
		.login_id(login_id)
		.permissions(entry.permissions)
		.build())
}
