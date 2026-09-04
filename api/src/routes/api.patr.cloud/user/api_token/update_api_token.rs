use models::{
	api::user::*,
	rbac::{ResourcePermissionTypeDiscriminant, WorkspacePermission},
};
use reqwest::StatusCode;
use rustis::commands::GenericCommands;

use crate::prelude::*;

pub async fn update_api_token(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: UpdateApiTokenPath { token_id },
				query: (),
				headers:
					UpdateApiTokenRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body:
					UpdateApiTokenRequestProcessed {
						token:
							UserApiTokenProcessed {
								name,
								permissions,
								token_nbf,
								token_exp,
								allowed_ips,
								created: _,
							},
					},
			},
		database,
		redis,
		client_ip: _,
		user_data,
		state: _,
	}: AuthenticatedAppRequest<'_, UpdateApiTokenRequest>,
) -> Result<AppResponse<UpdateApiTokenRequest>, ErrorType> {
	trace!("Updating API token: {}", token_id);

	if permissions.is_empty() {
		return Err(ErrorType::WrongParameters);
	}

	// The full object carries both bounds, so validate the window directly.
	if let (Some(nbf), Some(exp)) = (token_nbf, token_exp) {
		if nbf > exp {
			return Err(ErrorType::WrongParameters);
		}
	}

	// Normalize an empty whitelist to a clear request — semantically the
	// same as "no IP restriction", and stops the authenticator from rejecting
	// every request against a vacuous list.
	let allowed_ips = allowed_ips.filter(|ips| !ips.is_empty());

	let rows_updated = query!(
		r#"
		UPDATE
			user_api_token
		SET
			name = $1,
			token_nbf = $2,
			token_exp = $3,
			allowed_ips = $4
		WHERE
			token_id = $5 AND
			user_id = $6;
		"#,
		&*name,
		token_nbf,
		token_exp,
		allowed_ips.as_deref(),
		token_id as _,
		user_data.id as _,
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
			ErrorType::ApiTokenAlreadyExists
		}
		other => ErrorType::server_error(other),
	})?
	.rows_affected();

	// Bail before the perm-table DELETEs if the caller doesn't own this
	// token. Without this, the DELETE block below (keyed only on token_id)
	// would happily wipe another user's permission rows.
	if rows_updated == 0 {
		return Err(ErrorType::ApiTokenDoesNotExist);
	}

	trace!("API token updated");

	query!(
		r#"
		DELETE FROM
			user_api_token_workspace_super_admin
		WHERE
			token_id = $1;
		"#,
		token_id as _,
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		DELETE FROM
			user_api_token_resource_permissions_include
		WHERE
			token_id = $1;
		"#,
		token_id as _,
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		DELETE FROM
			user_api_token_resource_permissions_exclude
		WHERE
			token_id = $1;
		"#,
		token_id as _,
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		DELETE FROM
			user_api_token_resource_permissions_type
		WHERE
			token_id = $1;
		"#,
		token_id as _,
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		DELETE FROM
			user_api_token_workspace_permission_type
		WHERE
			token_id = $1;
		"#,
		token_id as _,
	)
	.execute(&mut **database)
	.await?;

	trace!("Existing permissions deleted");

	for (workspace_id, permission) in permissions {
		trace!("Inserting permission for workspace ID: `{workspace_id}`");

		let Some(user_permission) = user_data.permissions.get(&workspace_id) else {
			debug!("The user does not have any permissions on workspace ID: `{workspace_id}`");
			return Err(ErrorType::Unauthorized);
		};

		if !user_permission.is_superset_of(&permission, workspace_id) {
			debug!("The user does not have adequate permissions on workspace ID: `{workspace_id}`");
			return Err(ErrorType::Unauthorized);
		}

		match permission {
			WorkspacePermission::SuperAdmin => {
				trace!("Inserting permission as super admin");
				query!(
					r#"
					INSERT INTO
						user_api_token_workspace_permission_type(
							token_id,
							workspace_id,
							token_permission_type
						)
					VALUES
						(
							$1,
							$2,
							'super_admin'
						);
					"#,
					token_id as _,
					workspace_id as _,
				)
				.execute(&mut **database)
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
						(
							$1,
							$2,
							$3,
							DEFAULT
						);
					"#,
					token_id as _,
					user_data.id as _,
					workspace_id as _,
				)
				.execute(&mut **database)
				.await?;
			}
			WorkspacePermission::Member { permissions } => {
				trace!("Inserting permission as member");
				// The per-permission rows below FK onto this parent row, so it
				// has to land first. create_api_token does the same.
				query!(
					r#"
					INSERT INTO
						user_api_token_workspace_permission_type(
							token_id,
							workspace_id,
							token_permission_type
						)
					VALUES
						(
							$1,
							$2,
							'member'
						);
					"#,
					token_id as _,
					workspace_id as _,
				)
				.execute(&mut **database)
				.await?;

				for (permission_id, scopes) in permissions {
					// A grant at the workspace root is the legacy exclude-type
					// row with no exclusions; anything else is an include list.
					let scoped_to_root = scopes.contains(&workspace_id);
					let legacy_type = if scoped_to_root {
						ResourcePermissionTypeDiscriminant::Exclude
					} else {
						ResourcePermissionTypeDiscriminant::Include
					};
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
							(
								$1,
								$2,
								$3,
								$4,
								DEFAULT
							);
						"#,
						token_id as _,
						workspace_id as _,
						permission_id as _,
						legacy_type as _,
					)
					.execute(&mut **database)
					.await?;

					if !scoped_to_root {
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
								(
									$1,
									$2,
									$3,
									UNNEST($4::UUID[]),
									DEFAULT,
									DEFAULT
								);
							"#,
							token_id as _,
							workspace_id as _,
							permission_id as _,
							&scopes.into_iter().map(Into::into).collect::<Vec<_>>(),
						)
						.execute(&mut **database)
						.await?;
					}
				}
			}
		}
	}

	redis
		.del(redis::keys::permission_for_login_id(&token_id))
		.await?;

	AppResponse::builder()
		.body(UpdateApiTokenResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
