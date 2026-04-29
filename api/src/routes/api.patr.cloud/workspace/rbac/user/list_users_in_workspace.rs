use std::collections::BTreeMap;

use axum::http::StatusCode;
use models::{
	api::{user::BasicUserInfoSearchParams, workspace::rbac::user::*},
	utils::TotalCountHeader,
};

use crate::prelude::*;

/// The handler to list all users in the given workspace, along with their
/// roles.
pub async fn list_users_in_workspace(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListUsersInWorkspacePath { workspace_id },
				query:
					ListResourceQuery {
						sort: sort_order,
						search:
							BasicUserInfoSearchParams {
								username: username_filter,
								first_name: first_name_filter,
								last_name: last_name_filter,
							},
						count,
						page,
						additional_query: (),
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
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, ListUsersInWorkspaceRequest>,
) -> Result<AppResponse<ListUsersInWorkspaceRequest>, ErrorType> {
	info!("Listing all users in workspace `{workspace_id}`");

	let mut total_count = 0;
	let users = query!(
		r#"
		WITH matched_users AS (
			SELECT DISTINCT
				workspace_user.user_id
			FROM
				workspace_user
			INNER JOIN
				"user"
			ON
				workspace_user.user_id = "user".id
			WHERE
				workspace_user.workspace_id = $1 AND
				($2::TEXT IS NULL OR "user".username ILIKE '%' || $2::TEXT || '%') AND
				($3::TEXT IS NULL OR "user".first_name ILIKE '%' || $3::TEXT || '%') AND
				($4::TEXT IS NULL OR "user".last_name ILIKE '%' || $4::TEXT || '%')
		),
		users_page AS (
			SELECT
				user_id
			FROM
				matched_users
			ORDER BY
				user_id
			LIMIT $5
			OFFSET $6
		)
		SELECT
			workspace_user.user_id AS "user_id!",
			workspace_user.role_id AS "role_id!",
			(SELECT COUNT(*) FROM matched_users) AS "total_count!"
		FROM
			workspace_user
		INNER JOIN
			users_page
		ON
			users_page.user_id = workspace_user.user_id
		WHERE
			workspace_user.workspace_id = $1
		ORDER BY
			workspace_user.user_id,
			workspace_user.role_id;
		"#,
		workspace_id as _,
		username_filter,
		first_name_filter,
		last_name_filter,
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
