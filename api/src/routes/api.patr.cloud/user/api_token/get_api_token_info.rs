use std::collections::BTreeSet;

use models::{
	api::{user::*, workspace::rbac::user::RoleGrant},
	rbac::PermissionScope,
};
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
			role_id AS "role_id!: Uuid",
			(scope_id = workspace_id) AS "is_workspace_scope!",
			scope_id AS "scope_id!: Uuid"
		FROM
			api_token_role_binding
		WHERE
			token_id = $1
		ORDER BY
			workspace_id,
			role_id;
		"#,
		token_id as _,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.for_each(|row| {
		let scope = if row.is_workspace_scope {
			PermissionScope::Workspace
		} else {
			PermissionScope::Resources(BTreeSet::from([row.scope_id]))
		};
		let grants = token.data.grants.entry(row.workspace_id).or_default();
		if let Some(grant) = grants.iter_mut().find(|grant| grant.role_id == row.role_id) {
			grant.scope.union_with(&scope);
		} else {
			grants.push(RoleGrant {
				role_id: row.role_id,
				scope,
			});
		}
	});

	AppResponse::builder()
		.body(GetApiTokenInfoResponse { token })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
