use std::{collections::BTreeMap, net::IpAddr, ops::Sub, str::FromStr as _};

use models::{ActorData, RequestActorData};
use rand::{RngExt, distr::Alphanumeric};
use rustis::client::Client as RedisClient;
use sha2::{Digest as _, Sha256};
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

pub use self::cache::{
	mark_actor_stale,
	mark_all_stale,
	mark_login_stale,
	mark_token_stale,
	mark_workspace_stale,
};

/// Authenticate a bearer token and return who is making the request.
///
/// A token is either a JWT ([`authenticate_jwt`]) or an opaque API token or
/// service account token ([`authenticate_opaque_token`]). Both are cached —
/// JWTs by their login ID, opaque tokens by their hash — so a hit costs no
/// database round trip; a miss looks the login up, verifies it, loads its
/// permissions and caches the lot until the token expires or something marks
/// it stale (see [`cache`]).
///
/// `accepted_client_types` is who the caller serves. Kinds outside it aren't
/// even parsed: a JWT sent to a route that only takes API tokens is a
/// malformed API token as far as that route is concerned, and an opaque
/// token of a kind the route doesn't take is rejected before any work is done
/// on it.
pub async fn authenticate(
	database: &mut DatabaseConnection,
	redis: &mut RedisClient,
	config: &AppConfig,
	client_ip: IpAddr,
	token: &str,
	accepted_client_types: &[ActorClientType],
) -> Result<RequestActorData, ErrorType> {
	let accepts_jwt =
		accepted_client_types.contains(&ActorClientType::UserLogin(UserLoginType::WebLogin));
	let accepts_opaque_token = accepted_client_types
		.contains(&ActorClientType::UserLogin(UserLoginType::ApiToken)) ||
		accepted_client_types.contains(&ActorClientType::ServiceAccount);

	if accepts_jwt && let Ok(claims) = AccessTokenData::decode(token, &config.jwt_secret) {
		return authenticate_jwt(database, redis, claims).await;
	}

	if !accepts_opaque_token {
		warn!("Authentication header is not a JWT, and only JWTs are accepted here");
		return Err(ErrorType::MalformedAccessToken);
	}

	authenticate_opaque_token(database, redis, client_ip, token, accepted_client_types).await
}

/// Generates a new user API token: [`API_TOKEN_PREFIX`][constants::API_TOKEN_PREFIX],
/// [`OPAQUE_TOKEN_SECRET_LENGTH`][constants::OPAQUE_TOKEN_SECRET_LENGTH] random
/// base62 characters, and a checksum of both.
pub fn generate_api_token() -> String {
	let body = format!(
		"{}{}",
		constants::API_TOKEN_PREFIX,
		rand::rng()
			.sample_iter(Alphanumeric)
			.take(constants::OPAQUE_TOKEN_SECRET_LENGTH)
			.map(char::from)
			.collect::<String>()
	);

	format!(
		"{body}{:0>width$}",
		base62::encode_fmt(crc32fast::hash(body.as_bytes())),
		width = constants::OPAQUE_TOKEN_CHECKSUM_LENGTH,
	)
}

/// Generates a new service account token:
/// [`SERVICE_ACCOUNT_TOKEN_PREFIX`][constants::SERVICE_ACCOUNT_TOKEN_PREFIX],
/// [`OPAQUE_TOKEN_SECRET_LENGTH`][constants::OPAQUE_TOKEN_SECRET_LENGTH] random
/// base62 characters, and a checksum of both.
pub fn generate_service_account_token() -> String {
	let body = format!(
		"{}{}",
		constants::SERVICE_ACCOUNT_TOKEN_PREFIX,
		rand::rng()
			.sample_iter(Alphanumeric)
			.take(constants::OPAQUE_TOKEN_SECRET_LENGTH)
			.map(char::from)
			.collect::<String>()
	);

	format!(
		"{body}{:0>width$}",
		base62::encode_fmt(crc32fast::hash(body.as_bytes())),
		width = constants::OPAQUE_TOKEN_CHECKSUM_LENGTH,
	)
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

	let cache_key = redis::keys::auth_data_for_login_id(&claims.sub);
	let entry = if let Some(entry) = cache::read(redis, &cache_key).await {
		trace!("Cached auth data found for login `{}`", claims.sub);
		entry
	} else {
		let entry = web_dashboard::load_actor_auth_data(&mut *database, &claims.sub).await?;

		cache::write(
			redis,
			&cache_key,
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

/// Authenticate an opaque token: a user API token (`patr_at_…`) or a service
/// account token (`patr_sa_…`). The prefix says which, so a kind this route
/// doesn't take is turned away before anything is looked up. The token is
/// found by its SHA-256, in the cache and then in the database, so finding it
/// is itself the proof that it's genuine.
async fn authenticate_opaque_token(
	database: &mut DatabaseConnection,
	redis: &mut RedisClient,
	client_ip: IpAddr,
	token: &str,
	accepted_client_types: &[ActorClientType],
) -> Result<RequestActorData, ErrorType> {
	trace!("Parsing authentication header as an opaque token");

	let client_type = if token.starts_with(constants::API_TOKEN_PREFIX) {
		ActorClientType::UserLogin(UserLoginType::ApiToken)
	} else if token.starts_with(constants::SERVICE_ACCOUNT_TOKEN_PREFIX) {
		ActorClientType::ServiceAccount
	} else {
		warn!("Authentication header is neither a JWT nor an opaque token");
		return Err(ErrorType::MalformedApiToken);
	};

	// The prefix and secret, and the checksum of both. Only the checksum is
	// checked: anyone can compute a valid one, so length or character checks
	// wouldn't stop a crafted token.
	let Some((body, presented_checksum)) =
		token.split_at_checked(token.len() - constants::OPAQUE_TOKEN_CHECKSUM_LENGTH)
	else {
		warn!("Invalid opaque token: the checksum isn't on a character boundary");
		return Err(ErrorType::MalformedApiToken);
	};
	let calculated_checksum = format!(
		"{:0>width$}",
		base62::encode_fmt(crc32fast::hash(body.as_bytes())),
		width = constants::OPAQUE_TOKEN_CHECKSUM_LENGTH,
	);
	if presented_checksum != calculated_checksum {
		warn!("Invalid opaque token: the checksum doesn't match");
		return Err(ErrorType::MalformedApiToken);
	}

	if !accepted_client_types.contains(&client_type) {
		warn!("A `{client_type}` token was presented to a route that doesn't accept it");
		return Err(ErrorType::Unauthorized);
	}

	let token_hash = hex::encode(Sha256::digest(token));
	let cache_key = redis::keys::auth_data_for_token(&token_hash);

	let entry = if let Some(entry) = cache::read(redis, &cache_key).await {
		trace!("Cached auth data found for the token");
		entry
	} else {
		let (entry, ttl) = match client_type {
			ActorClientType::UserLogin(UserLoginType::ApiToken) => {
				api_token::load_actor_auth_data(&mut *database, &token_hash).await?
			}
			ActorClientType::ServiceAccount => {
				service_account::load_actor_auth_data(&mut *database, &token_hash).await?
			}
			ActorClientType::UserLogin(UserLoginType::WebLogin) => {
				unreachable!("only the opaque token prefixes are parsed above")
			}
		};

		cache::write(redis, &cache_key, &entry, ttl).await;
		entry
	};

	// Checked on every request, cache hit or not: for API tokens, the client's
	// IP.
	let (actor, created) = match entry.kind {
		ActorAuthDataCacheKind::ApiToken {
			email,
			first_name,
			last_name,
			created,
			allowed_ips,
		} => {
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
		ActorAuthDataCacheKind::ServiceAccount { name, created } => {
			(ActorData::ServiceAccount { name }, created)
		}
		ActorAuthDataCacheKind::WebLogin { .. } => {
			error!("A web login's cache entry was found under an opaque token's hash");
			return Err(ErrorType::server_error(
				"web login cached under a token hash",
			));
		}
	};

	Ok(RequestActorData::builder()
		.id(entry.actor_id)
		.actor(actor)
		.created(created)
		.login_id(entry.login_id)
		.permissions(entry.permissions)
		.build())
}
