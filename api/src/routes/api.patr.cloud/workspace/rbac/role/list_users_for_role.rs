use axum::http::StatusCode;
use models::{api::workspace::rbac::role::*, utils::TotalCountHeader};

use crate::prelude::*;

/// The handler to list all users for a role in the workspace. This will return
/// all the users that have the role in the workspace.
pub async fn list_users_for_role(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListUsersForRolePath {
					workspace_id,
					role_id,
				},
				query: Paginated {
					data: (),
					count,
					page,
				},
				headers:
					ListUsersForRoleRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ListUsersForRoleRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, ListUsersForRoleRequest>,
) -> Result<AppResponse<ListUsersForRoleRequest>, ErrorType> {
	info!("Listing all users for role: {}", role_id);

	let mut total_count = 0;
	let users = query!(
		r#"
        SELECT
            workspace_user.*,
            COUNT(*) OVER() AS "total_count!"
        FROM
            workspace_user
        INNER JOIN
            role
        ON
            role.id = workspace_user.role_id
        WHERE
            role.owner_id = $1
        LIMIT $2
        OFFSET $3;
        "#,
		workspace_id as _,
		count as i64,
		(page * count) as i64,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		total_count = row.total_count;
		row.user_id.into()
	})
	.collect();

	AppResponse::builder()
		.body(ListUsersForRoleResponse { users })
		.headers(ListUsersForRoleResponseHeaders {
			total_count: TotalCountHeader(total_count as usize),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
