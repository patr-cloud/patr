use models::{api::user::*, rbac::PermissionScope};
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
								super_admin_of,
								grants,
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

	if super_admin_of.is_empty() && grants.is_empty() {
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
			api_token_role_binding
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
			user_api_token_workspace_permission_type
		WHERE
			token_id = $1;
		"#,
		token_id as _,
	)
	.execute(&mut **database)
	.await?;

	trace!("Existing permissions deleted");

	// Super-admin entries: the DB itself enforces that only the workspace's
	// owner can mint these, via the FK to workspace(id, super_admin_id).
	for workspace_id in super_admin_of {
		trace!("Inserting super-admin entry for workspace ID: `{workspace_id}`");

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
		.await
		.map_err(|err| match err {
			sqlx::Error::Database(db_err) if db_err.is_foreign_key_violation() => {
				ErrorType::Unauthorized
			}
			other => ErrorType::server_error(other),
		})?;
	}

	// The ceiling rows. Validation is structural only — the composite FKs
	// pin the role and every scope to the named workspace. A ceiling above
	// the owner's current permissions is allowed: the intersection at auth
	// time clamps it, and a later promotion needs no re-mint.
	for (workspace_id, workspace_grants) in grants {
		trace!("Inserting ceiling rows for workspace ID: `{workspace_id}`");

		for grant in workspace_grants {
			if matches!(&grant.scope, PermissionScope::Resources(resources) if resources.is_empty())
			{
				return Err(ErrorType::WrongParameters);
			}
			let scope_ids = match &grant.scope {
				PermissionScope::Workspace => vec![workspace_id],
				PermissionScope::Resources(resources) => resources.iter().copied().collect(),
			};

			query!(
				r#"
				INSERT INTO
					api_token_role_binding(token_id, workspace_id, role_id, scope_id)
				SELECT
					$1, $2, $3, *
				FROM
					UNNEST($4::UUID[])
				ON CONFLICT
					(token_id, role_id, scope_id)
				DO NOTHING;
				"#,
				token_id as _,
				workspace_id as _,
				grant.role_id as _,
				&scope_ids as _,
			)
			.execute(&mut **database)
			.await
			.map_err(|err| match err {
				sqlx::Error::Database(db_err) if db_err.is_foreign_key_violation() => {
					match db_err.constraint() {
						Some("api_token_role_binding_fk_role_id_workspace_id") => {
							ErrorType::RoleDoesNotExist
						}
						_ => ErrorType::ResourceDoesNotExist,
					}
				}
				other => ErrorType::server_error(other),
			})?;
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
