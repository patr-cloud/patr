use models::{
	api::user::*,
	rbac::{ResourcePermissionType, ResourcePermissionTypeDiscriminant, WorkspacePermission},
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
						name,
						permissions,
						token_nbf,
						token_exp,
						allowed_ips,
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

	if name
		.as_ref()
		.map(|_| 0)
		.or(permissions.as_ref().map(|_| 0))
		.or(token_nbf.as_ref().map(|_| 0))
		.or(token_exp.as_ref().map(|_| 0))
		.or(allowed_ips.as_ref().map(|_| 0))
		.is_none()
	{
		debug!(
			"No parameters provided for updating API token: {}",
			token_id
		);
		return Err(ErrorType::WrongParameters);
	}

	// Normalize an empty whitelist to a clear request — semantically the
	// same as "no IP restriction", and stops the authenticator from rejecting
	// every request against a vacuous list.
	let allowed_ips = allowed_ips.map(|opt| opt.filter(|ips| !ips.is_empty()));

	// Tri-state per nullable field: None = keep existing, Some(None) = clear,
	// Some(Some(v)) = set. Encoded in SQL as CASE WHEN $clear THEN NULL etc.
	let nbf_clear = matches!(&token_nbf, Some(None));
	let nbf_value = token_nbf.flatten();
	let exp_clear = matches!(&token_exp, Some(None));
	let exp_value = token_exp.flatten();
	let ips_clear = matches!(&allowed_ips, Some(None));
	let ips_value = allowed_ips.flatten();

	let rows_updated = query!(
		r#"
		UPDATE
			user_api_token
		SET
			name = COALESCE($1, name),
			token_nbf = CASE
				WHEN $2 THEN NULL
				WHEN $3::TIMESTAMPTZ IS NOT NULL THEN $3
				ELSE token_nbf
			END,
			token_exp = CASE
				WHEN $4 THEN NULL
				WHEN $5::TIMESTAMPTZ IS NOT NULL THEN $5
				ELSE token_exp
			END,
			allowed_ips = CASE
				WHEN $6 THEN NULL
				WHEN $7::INET[] IS NOT NULL THEN $7
				ELSE allowed_ips
			END
		WHERE
			token_id = $8 AND
			user_id = $9;
		"#,
		name.as_deref(),
		nbf_clear,
		nbf_value,
		exp_clear,
		exp_value,
		ips_clear,
		ips_value.as_deref(),
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

	// Validate the merged nbf/exp window. A PATCH that only touches one side
	// can land the other in an unusable order against the existing value, so
	// re-read post-COALESCE and reject if the result is inverted. The tx
	// rolls the UPDATE back when we return Err.
	let bounds = query!(
		r#"
		SELECT
			token_nbf,
			token_exp
		FROM
			user_api_token
		WHERE
			token_id = $1;
		"#,
		token_id as _,
	)
	.fetch_one(&mut **database)
	.await?;

	if let (Some(nbf), Some(exp)) = (bounds.token_nbf, bounds.token_exp) {
		if nbf > exp {
			return Err(ErrorType::WrongParameters);
		}
	}

	trace!("API token updated");

	if let Some(permissions) = permissions {
		if permissions.is_empty() {
			return Err(ErrorType::WrongParameters);
		}

		trace!("Updating permissions for API token: {}", token_id);

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

			if !user_permission.is_superset_of(&permission) {
				debug!(
					"The user does not have adequate permissions on workspace ID: `{workspace_id}`"
				);
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
							ResourcePermissionTypeDiscriminant::from(&resource_permission) as _,
						)
						.execute(&mut **database)
						.await?;

						match resource_permission {
							ResourcePermissionType::Include(resource_ids) => {
								for resource_id in resource_ids {
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
												$4,
												DEFAULT,
												DEFAULT
											);
										"#,
										token_id as _,
										workspace_id as _,
										permission_id as _,
										resource_id as _,
									)
									.execute(&mut **database)
									.await?;
								}
							}
							ResourcePermissionType::Exclude(resource_ids) => {
								for resource_id in resource_ids {
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
											(
												$1,
												$2,
												$3,
												$4,
												DEFAULT,
												DEFAULT
											);
										"#,
										token_id as _,
										workspace_id as _,
										permission_id as _,
										resource_id as _,
									)
									.execute(&mut **database)
									.await?;
								}
							}
						}
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
