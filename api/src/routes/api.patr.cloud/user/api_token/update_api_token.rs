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
		config: _,
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

	query!(
		r#"
		UPDATE
			user_api_token
		SET
			name = COALESCE($1, name),
			token_nbf = COALESCE($2, token_nbf),
			token_exp = COALESCE($3, token_exp),
			allowed_ips = COALESCE($4, allowed_ips)
		WHERE
			token_id = $5 AND
			user_id = $6;
		"#,
		name.as_deref(),
		token_nbf,
		token_exp,
		allowed_ips.as_deref(),
		token_id as _,
		user_data.id as _,
	)
	.execute(&mut **database)
	.await?;

	trace!("API token updated");

	if let Some(permissions) = permissions {
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
