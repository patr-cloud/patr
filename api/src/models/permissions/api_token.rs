use std::{collections::BTreeMap, net::IpAddr};

use argon2::{Algorithm, Argon2, PasswordHash, PasswordVerifier as _, Version};
use models::{
	IdentityData,
	RequestUserData,
	rbac::{WorkspacePermission, intersect_workspace_permissions},
	utils::ClientType,
};
use rustis::client::Client as RedisClient;
use time::OffsetDateTime;

use crate::{prelude::*, utils::config::AppConfig};

pub(crate) async fn get_permissions(
	database: &mut DatabaseConnection,
	redis: &mut RedisClient,
	config: &AppConfig,
	client_ip: IpAddr,
	token: &str,
) -> Result<RequestUserData, ErrorType> {
	trace!("Parsing authentication header as an API token");
	let (refresh_token, login_id) = token
		.strip_prefix("patrv1.")
		.ok_or_else(|| {
			warn!("Invalid API token: missing `patrv1.` prefix");
			ErrorType::MalformedApiToken
		})?
		.split_once('.')
		.ok_or_else(|| {
			warn!("Invalid API token: missing refresh-token/login-id separator");
			ErrorType::MalformedApiToken
		})?;

	let refresh_token = Uuid::parse_str(refresh_token).map_err(|err| {
		warn!(
			"Invalid API token: refresh token is not a valid UUID: {}",
			err
		);
		ErrorType::MalformedApiToken
	})?;
	trace!("Refresh token parsed as UUID");

	let login_id = Uuid::parse_str(login_id).map_err(|err| {
		warn!("Invalid API token: login ID is not a valid UUID: {}", err);
		ErrorType::MalformedApiToken
	})?;
	trace!("Login ID parsed as UUID");

	// Resolve the token to an identity. The branches extract:
	// (identity_id, login_id, identity_created_at, token_hash, identity_data,
	// client_type)
	//
	// Service account tokens share the `patrv1.{refresh_token}.{id}` shape with
	// user API tokens, so we try user_api_token first, then fall back to
	// service_account. A UUIDv4 collision between user_login.login_id and
	// service_account.id is vanishingly unlikely, but even if it happened the
	// worst case is the SA can't authenticate (the user_api_token branch
	// matches first, then the hash check fails because the hashes don't match).
	// No unauthorized access is possible — just a soft-bricked SA.
	info!("Extracting information about API token");
	let (
		identity_id,
		resolved_login_id,
		identity_created_at,
		token_hash,
		identity,
		resolved_client_type,
	) = if let Some(token) = query!(
		r#"
		SELECT
			user_api_token.token_id AS "token_id: Uuid",
			user_api_token.user_id AS "user_id: Uuid",
			user_api_token.token_hash,
			user_api_token.token_nbf,
			user_api_token.token_exp,
			user_api_token.allowed_ips,
			user_api_token.revoked,
			"user".*
		FROM
			user_api_token
		INNER JOIN
			user_login
		ON
			user_api_token.token_id = user_login.login_id
		INNER JOIN
			"user"
		ON
			user_api_token.user_id = "user".id
		WHERE
			user_api_token.token_id = $1 AND
			user_login.login_type = 'api_token';
		"#,
		login_id as _
	)
	.fetch_optional(&mut *database) // What the actual fuck?
	.await?
	{
		trace!("Token extracted from database");

		if let Some(nbf) = token.token_nbf {
			trace!("Token has an NBF");
			if OffsetDateTime::now_utc() < nbf {
				info!("API token is not valid yet");
				return Err(ErrorType::AuthorizationTokenInvalid);
			}
		} else {
			trace!("Token does not have an NBF");
		}
		trace!("Token passed NBF check");

		if let Some(exp) = token.token_exp {
			trace!("Token has an EXP");
			if OffsetDateTime::now_utc() > exp {
				info!("API token has expired");
				return Err(ErrorType::AuthorizationTokenInvalid);
			}
		} else {
			trace!("Token does not have an EXP");
		}
		trace!("Token passed EXP check");

		if let Some(revoked) = token.revoked {
			trace!("Token has a revoked timestamp");
			if OffsetDateTime::now_utc() > revoked {
				info!("API token has been revoked");
				return Err(ErrorType::AuthorizationTokenInvalid);
			}
		} else {
			trace!("Token does not have a revoked timestamp");
		}
		trace!("Token passed revoked timestamp check");

		if let Some(allowed_ips) = token.allowed_ips &&
			!allowed_ips
				.iter()
				.any(|ip_network| ip_network.contains(client_ip))
		{
			info!("API token not accessed from an allowed IP Address");
			return Err(ErrorType::DisallowedIpAddressForApiToken);
		}

		(
			token.user_id,
			token.token_id,
			token.created,
			token.token_hash,
			IdentityData::User {
				email: token.email,
				first_name: token.first_name,
				last_name: token.last_name,
			},
			ClientType::ApiToken,
		)
	} else if let Some(service_account) = query!(
		r#"
		SELECT
			id AS "id: Uuid",
			name,
			token_hash,
			created
		FROM
			service_account
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
		login_id as _
	)
	.fetch_optional(&mut *database)
	.await?
	{
		trace!("Found service account token");

		// A service account holds a single, non-rotating credential rather than
		// a set of logins, so it acts as its own login ID.
		(
			service_account.id,
			service_account.id,
			service_account.created,
			service_account.token_hash,
			IdentityData::ServiceAccount {
				name: service_account.name,
			},
			ClientType::ServiceAccount,
		)
	} else {
		warn!("Token not found as a user API token or a service account");
		// No specific error for the token not being found, since we don't want
		// to leak information about whether a loginId is valid or if it's
		// expired
		return Err(ErrorType::AuthorizationTokenInvalid);
	};

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
	.verify_password(refresh_token.as_bytes(), &password_hash)
	.is_ok();

	if !success {
		warn!("Token has an invalid refresh token");
		return Err(ErrorType::AuthorizationTokenInvalid);
	}
	info!("Token valid");

	let permissions = super::get_permissions_for_identity(
		&mut *database,
		redis,
		&resolved_login_id,
		&identity_id,
		resolved_client_type,
	)
	.await?;

	Ok(RequestUserData::builder()
		.id(identity_id)
		.identity(identity)
		.client_type(resolved_client_type)
		.created(identity_created_at)
		.login_id(resolved_login_id)
		.permissions(permissions)
		.build())
}

/// Compute the effective permission map for an API token. A pure database
/// read — caching is the dispatcher's job.
///
/// 1. Reads the user's current role-derived permissions for the workspaces they are a member of
///    (the upper bound).
/// 2. Reads the token's declared permissions from `user_api_token_*` tables (the snapshot taken at
///    mint/patch time).
/// 3. Computes the intersection — anything the user has lost since the token was minted is dropped
///    from the token's effective scope.
/// 4. Rewrites the token's DB rows for any workspace whose intersection differs from the declared
///    rows, so subsequent reads (auth and `get_api_token_info`) see the converged state directly.
/// 5. Returns the effective map.
#[tracing::instrument(skip(db_connection))]
pub async fn get_permissions_for_api_token(
	db_connection: &mut DatabaseConnection,
	login_id: &Uuid,
	user_id: &Uuid,
) -> Result<BTreeMap<Uuid, WorkspacePermission>, ErrorType> {
	// User's current role-derived permissions (the upper bound for the
	// token). Read directly from the DB — the token's cache slot is keyed
	// on its own login_id, so reusing the user's cached perms doesn't apply.
	let mut user_permissions = BTreeMap::<Uuid, WorkspacePermission>::new();

	query!(
		r#"
		SELECT
			id AS "workspace_id!"
		FROM
			workspace
		WHERE
			super_admin_id = $1;
		"#,
		user_id as _,
	)
	.fetch_all(&mut *db_connection)
	.await?
	.into_iter()
	.map(|row| row.workspace_id)
	.for_each(|workspace_id| {
		user_permissions.insert(workspace_id.into(), WorkspacePermission::SuperAdmin);
	});

	query!(
		r#"
		SELECT
			role_binding.workspace_id AS "workspace_id!",
			role_permission.permission_id AS "permission_id!",
			role_binding.scope_id AS "scope_id!"
		FROM
			workspace_user
		INNER JOIN
			role_binding
		ON
			role_binding.actor_id = workspace_user.actor_id
		INNER JOIN
			role_permission
		ON
			role_permission.role_id = role_binding.role_id
		WHERE
			workspace_user.user_id = $1;
		"#,
		user_id as _,
	)
	.fetch_all(&mut *db_connection)
	.await?
	.into_iter()
	.for_each(|row| {
		let permissions = user_permissions
			.entry(row.workspace_id.into())
			.or_insert_with(|| WorkspacePermission::Member {
				permissions: BTreeMap::new(),
			});

		let WorkspacePermission::Member { permissions } = permissions else {
			// Super admin of this workspace — bindings are redundant.
			return;
		};

		// A scope is just a resource id; the workspace's own id is the root
		// and covers everything under it.
		permissions
			.entry(row.permission_id.into())
			.or_default()
			.insert(row.scope_id.into());
	});

	// Token's declared permissions (the snapshot at mint/patch time).
	let mut token_permissions = BTreeMap::<Uuid, WorkspacePermission>::new();

	query!(
		r#"
		SELECT
			workspace_id AS "workspace_id!"
		FROM
			user_api_token_workspace_super_admin
		WHERE
			token_id = $1;
		"#,
		login_id as _,
	)
	.fetch_all(&mut *db_connection)
	.await?
	.into_iter()
	.map(|row| row.workspace_id)
	.for_each(|workspace_id| {
		token_permissions.insert(workspace_id.into(), WorkspacePermission::SuperAdmin);
	});

	// The token's declared ceiling: its own (permission, scope) rows.
	query!(
		r#"
		SELECT
			user_api_token_permission_binding.workspace_id AS "workspace_id",
			user_api_token_permission_binding.permission_id AS "permission_id",
			user_api_token_permission_binding.scope_id AS "scope_id"
		FROM
			user_api_token_permission_binding
		WHERE
			user_api_token_permission_binding.token_id = $1;
		"#,
		login_id as _,
	)
	.fetch_all(&mut *db_connection)
	.await?
	.into_iter()
	.for_each(|row| {
		let permissions = token_permissions
			.entry(row.workspace_id.into())
			.or_insert_with(|| WorkspacePermission::Member {
				permissions: BTreeMap::new(),
			});

		let WorkspacePermission::Member { permissions } = permissions else {
			// Super admin of this workspace — declared rows are redundant.
			return;
		};

		// A scope is just a resource id; the workspace's own id is the root
		// and covers everything under it.
		permissions
			.entry(row.permission_id.into())
			.or_default()
			.insert(row.scope_id.into());
	});

	let effective_permissions =
		intersect_workspace_permissions(&token_permissions, &user_permissions);

	Ok(effective_permissions)
}
