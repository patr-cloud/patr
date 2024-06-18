use axum::http::StatusCode;
use models::api::{user::*, workspace::Workspace, WithId};

use crate::prelude::*;

pub async fn list_workspaces(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListUserWorkspacesPath,
				query: (),
				headers:
					ListUserWorkspacesRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ListUserWorkspacesRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data,
	}: AuthenticatedAppRequest<'_, ListUserWorkspacesRequest>,
) -> Result<AppResponse<ListUserWorkspacesRequest>, ErrorType> {
	info!("Listing all user workspaces");

	let workspaces = query!(
		r#"
		SELECT DISTINCT
			workspace.id,
			workspace.name::TEXT AS "name!",
			workspace.super_admin_id
		FROM
			workspace
		LEFT JOIN
			workspace_user
		ON
			workspace.id = workspace_user.workspace_id
		WHERE
			(
				workspace.super_admin_id = $1 OR
				workspace_user.user_id = $1
			) AND
			workspace.deleted IS NULL;
		"#,
		user_data.id as _,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		WithId::new(
			row.id,
			Workspace {
				name: row.name,
				super_admin_id: row.super_admin_id.into(),
			},
		)
	})
	.collect();

	AppResponse::builder()
		.body(ListUserWorkspacesResponse { workspaces })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
