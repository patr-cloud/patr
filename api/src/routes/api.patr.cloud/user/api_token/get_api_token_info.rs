use std::collections::{BTreeMap, BTreeSet};

use models::{
	api::user::*,
	rbac::{ResourcePermissionType, WorkspacePermission},
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
		redis: _,
		client_ip: _,
		user_data,
		config: _,
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
				permissions: Default::default(),
				token_nbf: row.token_nbf,
				token_exp: row.token_exp,
				allowed_ips: row.allowed_ips,
				created: row.created,
			},
		)
	})?;

	trace!("Basic token info fetched");

	let mut permissions = BTreeMap::<Uuid, WorkspacePermission>::new();

	query!(
		r#"
		SELECT
			workspace_id
		FROM
			user_api_token_workspace_super_admin
		WHERE
			token_id = $1;
		"#,
		token_id as _
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.for_each(|row| {
		permissions.insert(row.workspace_id.into(), WorkspacePermission::SuperAdmin);
	});

	trace!("Super-admin permissions fetched");

	// Once all super-admins are added, add the excludes, then remove the includes
	query!(
		r#"
		SELECT
            workspace_id,
            resource_id,
            permission_id
		FROM
			user_api_token_resource_permissions_exclude
        WHERE
			token_id = $1;
		"#,
		token_id as _
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.for_each(|row| {
		let permission = permissions
			.entry(row.workspace_id.into())
			.or_insert_with(|| WorkspacePermission::Member {
				permissions: BTreeMap::new(),
			});
		match permission {
			WorkspacePermission::SuperAdmin => {
				error!("SuperAdmin found when Member expected. This shouldn't be possible!");
			}
			WorkspacePermission::Member { permissions } => {
				let permission_type = permissions
					.entry(row.permission_id.into())
					.or_insert_with(|| ResourcePermissionType::Exclude(BTreeSet::new()));
				match permission_type {
					ResourcePermissionType::Include(_) => {
						error!(
							"Found include permissions before include is even called. This should be possible!"
						);
					}
					ResourcePermissionType::Exclude(resources) => {
						resources.insert(row.resource_id.into());
					}
				}
			}
		}
	});

	trace!("Exclude permissions fetched");

	query!(
		r#"
		SELECT
            workspace_id,
            resource_id,
            permission_id
		FROM
			user_api_token_resource_permissions_include
        WHERE
			token_id = $1;
		"#,
		token_id as _
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.for_each(|row| {
		let permission = permissions
			.entry(row.workspace_id.into())
			.or_insert_with(|| WorkspacePermission::Member {
				permissions: BTreeMap::new(),
			});
		match permission {
			WorkspacePermission::SuperAdmin => {
				error!("SuperAdmin found when Member expected. This shouldn't be possible!");
			}
			WorkspacePermission::Member { permissions } => {
				permissions
					.entry(row.permission_id.into())
					.or_insert_with(|| ResourcePermissionType::Include(BTreeSet::new()))
					.insert(row.resource_id.into());
			}
		}
	});

	trace!("Include permissions fetched");

	token.data.permissions = permissions;

	AppResponse::builder()
		.body(GetApiTokenInfoResponse { token })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
