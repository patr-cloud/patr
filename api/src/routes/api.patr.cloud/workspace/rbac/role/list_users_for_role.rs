use axum::http::StatusCode;
use models::{
	api::{
		WithId,
		workspace::rbac::{
			role::*,
			user::{WorkspaceUserInfo, WorkspaceUserInfoSearchParams},
		},
	},
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
					ListResourceQueryProcessed {
						sort: sort_order,
						search:
							WorkspaceUserInfoSearchParams {
								email: email_filter,
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
		WITH role_holder AS (
			SELECT DISTINCT
				workspace_user.user_id
			FROM
				role_binding
			INNER JOIN
				workspace_user
			ON
				workspace_user.actor_id = role_binding.actor_id
			WHERE
				role_binding.workspace_id = $1 AND
				role_binding.role_id = $2
		)
		SELECT
			role_holder.user_id AS "user_id!: Uuid",
			"user".first_name,
			"user".last_name,
			"user".email,
			COUNT(*) OVER() AS "total_count!"
		FROM
			role_holder
		INNER JOIN
			"user"
		ON
			role_holder.user_id = "user".id
		WHERE
			($3::TEXT IS NULL OR "user".email ILIKE '%' || $3::TEXT || '%') AND
			($4::TEXT IS NULL OR "user".first_name ILIKE '%' || $4::TEXT || '%') AND
			($5::TEXT IS NULL OR "user".last_name ILIKE '%' || $5::TEXT || '%')
		ORDER BY
			role_holder.user_id
		LIMIT $6
		OFFSET $7;
		"#,
		workspace_id as _,
		role_id as _,
		email_filter,
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
		WithId::new(
			row.user_id,
			WorkspaceUserInfo {
				first_name: row.first_name,
				last_name: row.last_name,
				email: row.email,
			},
		)
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
