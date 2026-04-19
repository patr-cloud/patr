use std::{
	collections::{BTreeMap, BTreeSet},
	net::IpAddr,
};

use argon2::{Algorithm, Argon2, PasswordHash, PasswordVerifier as _, Version};
use models::{
	IdentityData,
	RequestUserData,
	rbac::{
		ResourcePermissionType,
		ResourcePermissionTypeDiscriminant,
		WorkspacePermission,
		intersect_workspace_permissions,
	},
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
				username: token.username,
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

/// Compute the effective permission map for an API token:
///
/// 1. Reads the user's current role-derived permissions for the workspaces they
///    are a member of (the upper bound).
/// 2. Reads the token's declared permissions from `user_api_token_*` tables
///    (the snapshot taken at mint/patch time).
/// 3. Computes the intersection — anything the user has lost since the token
///    was minted is dropped from the token's effective scope.
/// 4. Rewrites the token's DB rows for any workspace whose intersection differs
///    from the declared rows, so subsequent reads (auth and
///    `get_api_token_info`) see the converged state directly.
///
/// Caching is the caller's job — see [`get_permissions_for_identity`][1].
///
/// [1]: super::get_permissions_for_identity
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

	query!(
		r#"
		SELECT
			user_api_token_resource_permissions_type.workspace_id AS "workspace_id!",
			user_api_token_resource_permissions_type.permission_id AS "permission_id!",
			user_api_token_resource_permissions_exclude.resource_id AS "resource_id?"
		FROM
			user_api_token_resource_permissions_type
		LEFT JOIN
			user_api_token_resource_permissions_exclude
		ON
			user_api_token_resource_permissions_exclude.token_id =
				user_api_token_resource_permissions_type.token_id
		WHERE
			user_api_token_resource_permissions_type.token_id = $1 AND
			user_api_token_resource_permissions_type.resource_permission_type = 'exclude';
		"#,
		login_id as _,
	)
	.fetch_all(&mut *db_connection)
	.await?
	.into_iter()
	.map(|row| (row.workspace_id, row.permission_id, row.resource_id))
	.for_each(|(workspace_id, permission_id, resource_id)| {
		let permissions = token_permissions
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
			user_api_token_resource_permissions_type.workspace_id AS "workspace_id!",
			user_api_token_resource_permissions_type.permission_id AS "permission_id!",
			user_api_token_resource_permissions_include.resource_id AS "resource_id?"
		FROM
			user_api_token_resource_permissions_type
		LEFT JOIN
			user_api_token_resource_permissions_include
		ON
			user_api_token_resource_permissions_include.token_id =
				user_api_token_resource_permissions_type.token_id
		WHERE
			user_api_token_resource_permissions_type.token_id = $1 AND
			user_api_token_resource_permissions_type.resource_permission_type = 'include';
		"#,
		login_id as _,
	)
	.fetch_all(&mut *db_connection)
	.await?
	.into_iter()
	.map(|row| (row.workspace_id, row.permission_id, row.resource_id))
	.for_each(|(workspace_id, permission_id, resource_id)| {
		let permissions = token_permissions
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

	let effective_permissions =
		intersect_workspace_permissions(&token_permissions, &user_permissions);

	// Write-back: rewrite the token's rows for any workspace whose effective
	// permissions differ from the declared ones. Steady-state requests where
	// nothing changed skip the writes entirely.
	if effective_permissions != token_permissions {
		let changed_workspaces = token_permissions
			.keys()
			.chain(effective_permissions.keys())
			.copied()
			.filter(|workspace_id| {
				token_permissions.get(workspace_id) != effective_permissions.get(workspace_id)
			})
			.collect::<BTreeSet<_>>();

		for workspace_id in &changed_workspaces {
			// DELETE in FK-safe order. The four resource/perm-type tables
			// reference the workspace_permission_type parent row, so the
			// parent comes last.
			query!(
				r#"
				DELETE FROM
					user_api_token_resource_permissions_include
				WHERE
					token_id = $1 AND
					workspace_id = $2;
				"#,
				login_id as _,
				workspace_id as _,
			)
			.execute(&mut *db_connection)
			.await?;

			query!(
				r#"
				DELETE FROM
					user_api_token_resource_permissions_exclude
				WHERE
					token_id = $1 AND
					workspace_id = $2;
				"#,
				login_id as _,
				workspace_id as _,
			)
			.execute(&mut *db_connection)
			.await?;

			query!(
				r#"
				DELETE FROM
					user_api_token_resource_permissions_type
				WHERE
					token_id = $1 AND
					workspace_id = $2;
				"#,
				login_id as _,
				workspace_id as _,
			)
			.execute(&mut *db_connection)
			.await?;

			query!(
				r#"
				DELETE FROM
					user_api_token_workspace_super_admin
				WHERE
					token_id = $1 AND
					workspace_id = $2;
				"#,
				login_id as _,
				workspace_id as _,
			)
			.execute(&mut *db_connection)
			.await?;

			query!(
				r#"
				DELETE FROM
					user_api_token_workspace_permission_type
				WHERE
					token_id = $1 AND
					workspace_id = $2;
				"#,
				login_id as _,
				workspace_id as _,
			)
			.execute(&mut *db_connection)
			.await?;

			// INSERT the new shape if the workspace survived intersection.
			// Mirrors the per-workspace INSERT block in
			// `create_api_token::create_api_token`.
			let Some(permission) = effective_permissions.get(workspace_id) else {
				continue;
			};

			match permission {
				WorkspacePermission::SuperAdmin => {
					query!(
						r#"
						INSERT INTO
							user_api_token_workspace_permission_type(
								token_id,
								workspace_id,
								token_permission_type
							)
						VALUES
							($1, $2, 'super_admin');
						"#,
						login_id as _,
						workspace_id as _,
					)
					.execute(&mut *db_connection)
					.await?;

					query!(
						r#"
						INSERT INTO
							user_api_token_workspace_super_admin(
								token_id,
								user_id,
								workspace_id,
								token_permission_type
							)
						VALUES
							($1, $2, $3, DEFAULT);
						"#,
						login_id as _,
						user_id as _,
						workspace_id as _,
					)
					.execute(&mut *db_connection)
					.await?;
				}
				WorkspacePermission::Member { permissions } => {
					query!(
						r#"
						INSERT INTO
							user_api_token_workspace_permission_type(
								token_id,
								workspace_id,
								token_permission_type
							)
						VALUES
							($1, $2, 'member');
						"#,
						login_id as _,
						workspace_id as _,
					)
					.execute(&mut *db_connection)
					.await?;

					for (permission_id, resource_permission) in permissions {
						query!(
							r#"
							INSERT INTO
								user_api_token_resource_permissions_type(
									token_id,
									workspace_id,
									permission_id,
									resource_permission_type,
									token_permission_type
								)
							VALUES
								($1, $2, $3, $4, DEFAULT);
							"#,
							login_id as _,
							workspace_id as _,
							permission_id as _,
							ResourcePermissionTypeDiscriminant::from(resource_permission) as _,
						)
						.execute(&mut *db_connection)
						.await?;

						match resource_permission {
							ResourcePermissionType::Include(resource_ids) => {
								query!(
									r#"
									INSERT INTO
										user_api_token_resource_permissions_include(
											token_id,
											workspace_id,
											permission_id,
											resource_id,
											resource_deleted,
											permission_type
										)
									VALUES
										($1, $2, $3, UNNEST($4::UUID[]), DEFAULT, DEFAULT);
									"#,
									login_id as _,
									workspace_id as _,
									permission_id as _,
									&resource_ids
										.iter()
										.map(|id| (*id).into())
										.collect::<Vec<_>>(),
								)
								.execute(&mut *db_connection)
								.await?;
							}
							ResourcePermissionType::Exclude(resource_ids) => {
								query!(
									r#"
									INSERT INTO
										user_api_token_resource_permissions_exclude(
											token_id,
											workspace_id,
											permission_id,
											resource_id,
											resource_deleted,
											permission_type
										)
									VALUES
										($1, $2, $3, UNNEST($4::UUID[]), DEFAULT, DEFAULT);
									"#,
									login_id as _,
									workspace_id as _,
									permission_id as _,
									&resource_ids
										.iter()
										.map(|id| (*id).into())
										.collect::<Vec<_>>(),
								)
								.execute(&mut *db_connection)
								.await?;
							}
						}
					}
				}
			}
		}
	}

	Ok(effective_permissions)
}
