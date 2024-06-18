use std::collections::BTreeMap;

use axum::http::StatusCode;
use models::{api::workspace::rbac::user::*, utils::TotalCountHeader};

use crate::prelude::*;

/// The handler to list all users in the given workspace, along with their
/// roles.
pub async fn list_users_in_workspace(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListUsersInWorkspacePath { workspace_id },
				query: Paginated {
					data: (),
					count,
					page,
				},
				headers:
					ListUsersInWorkspaceRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ListUsersInWorkspaceRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, ListUsersInWorkspaceRequest>,
) -> Result<AppResponse<ListUsersInWorkspaceRequest>, ErrorType> {
	info!("Listing all users in workspace `{workspace_id}`");

	let mut total_count = 0;
	let users = query!(
		r#"
        SELECT
            *,
            COUNT(*) OVER() AS "total_count!"
        FROM
            workspace_user
        WHERE
            workspace_id = $1
        ORDER BY
            user_id, role_id
        LIMIT $2
        OFFSET $3;
        "#,
		workspace_id as _,
		count as i64,
		(count * page) as i64,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.fold(BTreeMap::<Uuid, Vec<Uuid>>::new(), |mut users, row| {
		total_count = row.total_count;
		users
			.entry(row.user_id.into())
			.or_default()
			.push(row.role_id.into());
		users
	});

	AppResponse::builder()
		.body(ListUsersInWorkspaceResponse { users })
		.headers(ListUsersInWorkspaceResponseHeaders {
			total_count: TotalCountHeader(total_count as _),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
