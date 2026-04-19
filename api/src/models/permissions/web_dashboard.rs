use std::{
	collections::{BTreeMap, BTreeSet},
	net::IpAddr,
	ops::Sub,
};

use jsonwebtoken::{DecodingKey, TokenData, Validation};
use models::{
	IdentityData,
	RequestUserData,
	rbac::{ResourcePermissionType, WorkspacePermission},
	utils::ClientType,
};
use rustis::client::Client as RedisClient;
use time::OffsetDateTime;

use crate::{models::access_token_data::AccessTokenData, prelude::*, utils::config::AppConfig};

pub(crate) async fn get_permissions(
	database: &mut DatabaseConnection,
	redis: &mut RedisClient,
	config: &AppConfig,
	_client_ip: IpAddr,
	token: &str,
) -> Result<RequestUserData, ErrorType> {
	trace!("Parsing authentication header as a JWT");

	let TokenData {
		header: _,
		claims: AccessTokenData {
			iss,
			sub,
			aud,
			exp,
			nbf,
			iat: _,
			jti,
		},
	} = jsonwebtoken::decode(
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
	)
	.map_err(|err| {
		warn!("Invalid JWT provided: {}", err);
		ErrorType::MalformedAccessToken
	})?;
	trace!("Authentication header is a valid JWT");

	if iss != constants::JWT_ISSUER {
		warn!("Invalid JWT issuer: {}", iss);
		return Err(ErrorType::MalformedAccessToken);
	}
	trace!("JWT issuer valid");

	// The token should have been issued within the last `REFRESH_TOKEN_VALIDITY`
	// duration
	if OffsetDateTime::now_utc().sub(jti.get_timestamp().ok_or(ErrorType::MalformedAccessToken)?) >
		AccessTokenData::REFRESH_TOKEN_VALIDITY
	{
		warn!("JWT is too old");
		return Err(ErrorType::AuthorizationTokenInvalid);
	}
	trace!("JWT JTI valid");

	if OffsetDateTime::now_utc() < nbf {
		warn!("JWT is not valid yet");
		return Err(ErrorType::AuthorizationTokenInvalid);
	}
	trace!("JWT NBF valid");

	if OffsetDateTime::now_utc() > exp {
		warn!("JWT has expired");
		return Err(ErrorType::AuthorizationTokenInvalid);
	}
	trace!("JWT EXP valid");

	let Some(user) = query! {
		r#"
		SELECT
			"user".*
		FROM
			"user"
		INNER JOIN
			user_login
		ON
			"user".id = user_login.user_id
		INNER JOIN
			web_login
		ON
			user_login.login_id = web_login.login_id
		WHERE
			user_login.login_id = $1 AND
			user_login.login_type = 'web_login';
		"#,
		sub as _
	}
	.fetch_optional(&mut *database)
	.await?
	else {
		warn!("web login not found");
		// No specific error for API token not found, since we don't want to leak
		// information about whether a loginId is valid or if it's expired
		return Err(ErrorType::AuthorizationTokenInvalid);
	};
	trace!("Web login exists in the database");

	// Note: `web_login.token_expiry` is the refresh token's lifetime, not the
	// access token's. Access token validity is gated by the JWT's own `exp`
	// claim (checked above). Re-checking `token_expiry` here would prevent a
	// fresh JWT (post-refresh) from authenticating until the entire session
	// is renewed, and would also keep an old, expired JWT alive as long as
	// the session itself was still fresh. Both are wrong.

	if !aud
		.clone()
		.into_iter()
		.any(|item| item == constants::PATR_JWT_AUDIENCE)
	{
		warn!(
			"Invalid JWT audience: `{}`",
			match aud {
				OneOrMore::One(aud) => aud,
				OneOrMore::Multiple(aud) => format!("[{}]", aud.join(", ")),
			}
		);
		return Err(ErrorType::MalformedAccessToken);
	}

	let permissions = super::get_permissions_for_identity(
		&mut *database,
		redis,
		&sub,
		&user.id.into(),
		ClientType::WebDashboard,
	)
	.await?;

	Ok(RequestUserData::builder()
		.id(user.id)
		.identity(IdentityData::User {
			username: user.username,
			first_name: user.first_name,
			last_name: user.last_name,
		})
		.client_type(ClientType::WebDashboard)
		.created(user.created)
		.login_id(sub)
		.permissions(permissions)
		.build())
}

/// Compute the permission map for a web-login session: the user's current
/// role-derived permissions, read from the database (workspace ownership +
/// per-role includes/excludes for the workspaces they're a member of).
///
/// Caching is the caller's job — see [`get_permissions_for_identity`][1].
///
/// [1]: super::get_permissions_for_identity
#[tracing::instrument(skip(db_connection))]
pub async fn get_permissions_for_web_login(
	db_connection: &mut DatabaseConnection,
	user_id: &Uuid,
) -> Result<BTreeMap<Uuid, WorkspacePermission>, ErrorType> {
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
			workspace_user.workspace_id AS "workspace_id!",
			role_resource_permissions_type.permission_id AS "permission_id!",
			role_resource_permissions_exclude.resource_id AS "resource_id?"
		FROM
			workspace_user
		INNER JOIN
			role_resource_permissions_type
		ON
			role_resource_permissions_type.role_id = workspace_user.role_id AND
			role_resource_permissions_type.permission_type = 'exclude'
		LEFT JOIN
			role_resource_permissions_exclude
		ON
			role_resource_permissions_exclude.role_id = workspace_user.role_id
		WHERE
			workspace_user.user_id = $1;
		"#,
		user_id as _,
	)
	.fetch_all(&mut *db_connection)
	.await?
	.into_iter()
	.map(|row| (row.workspace_id, row.permission_id, row.resource_id))
	.for_each(|(workspace_id, permission_id, resource_id)| {
		let permissions = user_permissions
			.entry(workspace_id.into())
			.or_insert_with(|| WorkspacePermission::Member {
				permissions: BTreeMap::new(),
			});

		match permissions {
			WorkspacePermission::SuperAdmin => {
				error!("SuperAdmin found when Member expected. This shouldn't be possible!");
			}
			WorkspacePermission::Member { permissions } => {
				let permission_type = permissions
					.entry(permission_id.into())
					.or_insert_with(|| ResourcePermissionType::Exclude(BTreeSet::new()));
				match permission_type {
					ResourcePermissionType::Include(_) => {
						error!(
							"Found include permissions before include is even called. This should be possible!"
						);
					}
					ResourcePermissionType::Exclude(resources) => {
						let Some(resource_id) = resource_id else {
							return;
						};

						resources.insert(resource_id.into());
					}
				}
			}
		}
	});

	query!(
		r#"
		SELECT
			workspace_user.workspace_id AS "workspace_id!",
			role_resource_permissions_type.permission_id AS "permission_id!",
			role_resource_permissions_include.resource_id AS "resource_id?"
		FROM
			workspace_user
		INNER JOIN
			role_resource_permissions_type
		ON
			role_resource_permissions_type.role_id = workspace_user.role_id AND
			role_resource_permissions_type.permission_type = 'include'
		LEFT JOIN
			role_resource_permissions_include
		ON
			role_resource_permissions_include.role_id = workspace_user.role_id
		WHERE
			workspace_user.user_id = $1;
		"#,
		user_id as _,
	)
	.fetch_all(&mut *db_connection)
	.await?
	.into_iter()
	.map(|row| (row.workspace_id, row.permission_id, row.resource_id))
	.for_each(|(workspace_id, permission_id, resource_id)| {
		let permissions = user_permissions
			.entry(workspace_id.into())
			.or_insert_with(|| WorkspacePermission::Member {
				permissions: BTreeMap::new(),
			});

		let Some(resource_id) = resource_id else {
			return;
		};

		match permissions {
			WorkspacePermission::SuperAdmin => {
				error!("SuperAdmin found when Member expected. This shouldn't be possible!");
			}
			WorkspacePermission::Member { permissions } => {
				let permission_type = permissions
					.entry(permission_id.into())
					.or_insert_with(|| ResourcePermissionType::Include(BTreeSet::new()));
				match permission_type {
					ResourcePermissionType::Include(resources) => {
						resources.insert(resource_id.into());
					}
					ResourcePermissionType::Exclude(resources) => {
						resources.remove(&resource_id.into());
					}
				}
			}
		}
	});

	Ok(user_permissions)
}
