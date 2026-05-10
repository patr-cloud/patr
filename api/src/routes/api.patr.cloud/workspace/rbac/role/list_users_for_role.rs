use axum::http::StatusCode;
use models::{
	api::{user::BasicUserInfoSearchParams, workspace::rbac::role::*},
	utils::TotalCountHeader,
};

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
				query:
					ListResourceQuery {
						sort: sort_order,
						search:
							BasicUserInfoSearchParams {
								first_name: first_name_filter,
								last_name: last_name_filter,
							},
						count,
						page,
						additional_query: (),
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
		user_data: _,
		state: _,
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
			"user"
		ON
			workspace_user.user_id = "user".id
		WHERE
			workspace_id = $1 AND
			($2::TEXT IS NULL OR "user".first_name ILIKE '%' || $2::TEXT || '%') AND
			($3::TEXT IS NULL OR "user".last_name ILIKE '%' || $3::TEXT || '%')
		ORDER BY
			workspace_user.user_id
		LIMIT $4
		OFFSET $5;
		"#,
		workspace_id as _,
		first_name_filter,
		last_name_filter,
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
			total_count: TotalCountHeader(total_count as _),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
