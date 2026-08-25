use std::collections::BTreeSet;

use models::api::user::*;
use reqwest::StatusCode;

use crate::prelude::*;

pub async fn get_api_token_info(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetApiTokenInfoPath { token_id },
				query: (),
				headers:
					GetApiTokenInfoRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: GetApiTokenInfoRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		user_data,
		state: _,
	}: AuthenticatedAppRequest<'_, GetApiTokenInfoRequest>,
) -> Result<AppResponse<GetApiTokenInfoRequest>, ErrorType> {
	trace!("Getting info for API token: {}", token_id);

	let mut token = query!(
		r#"
		SELECT
			token_id,
			name,
			token_nbf,
			token_exp,
			allowed_ips,
			created
		FROM
			user_api_token
		WHERE
			token_id = $1 AND
			user_id = $2 AND
			revoked IS NULL;
		"#,
		token_id as _,
		user_data.id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ApiTokenDoesNotExist)
	.map(|row| {
		WithId::new(
			row.token_id,
			UserApiToken {
				name: row.name,
				super_admin_of: Default::default(),
				grants: Default::default(),
				token_nbf: row.token_nbf,
				token_exp: row.token_exp,
				allowed_ips: row.allowed_ips,
				created: row.created,
			},
		)
	})?;

	trace!("Basic token info fetched");

	// The declared ceiling. Effective permissions are the ceiling
	// intersected with the owner's current permissions, computed at auth —
	// the declaration is what the owner can inspect and edit.
	token.data.super_admin_of = query!(
		r#"
		SELECT
			workspace_id AS "workspace_id!: Uuid"
		FROM
			user_api_token_workspace_super_admin
		WHERE
			token_id = $1;
		"#,
		token_id as _,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| row.workspace_id)
	.collect();

	query!(
		r#"
		SELECT
			workspace_id AS "workspace_id!: Uuid",
			permission_id AS "permission_id!: Uuid",
			scope_id AS "scope_id!: Uuid"
		FROM
			user_api_token_permission_binding
		WHERE
			token_id = $1
		ORDER BY
			workspace_id,
			permission_id;
		"#,
		token_id as _,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.for_each(|row| {
		// One grant per ceiling row; the workspace's own id is the root.
		token
			.data
			.grants
			.entry(row.workspace_id)
			.or_default()
			.push(PermissionGrant {
				permission_id: row.permission_id,
				resource_id: row.scope_id,
			});
	});

	AppResponse::builder()
		.body(GetApiTokenInfoResponse { token })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
