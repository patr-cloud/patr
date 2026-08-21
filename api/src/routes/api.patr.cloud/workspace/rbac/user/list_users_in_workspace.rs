use axum::http::StatusCode;
use models::{
	api::{
		WithId,
		workspace::rbac::user::{WorkspaceUserInfo, WorkspaceUserInfoSearchParams, *},
	},
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

	// The owner holds super-admin rights directly on the workspace rather than
	// through a role, so they have no `workspace_user` rows. UNION them in
	// here — they're a member as far as anyone reading this list is
	// concerned, and folding them in keeps pagination and total_count honest.
	let mut total_count = 0;
	let users = query!(
		r#"
		WITH members AS (
			SELECT DISTINCT
				workspace_user.user_id
			FROM
				workspace_user
			WHERE
				workspace_user.workspace_id = $1
			UNION
			SELECT
				workspace.super_admin_id AS user_id
			FROM
				workspace
			WHERE
				workspace.id = $1
		),
		matched_users AS (
			SELECT
				members.user_id,
				"user".first_name,
				"user".last_name,
				"user".email,
				(workspace.id IS NOT NULL) AS is_owner
			FROM
				members
			INNER JOIN
				"user"
			ON
				members.user_id = "user".id
			LEFT JOIN
				workspace
			ON
				workspace.id = $1 AND
				workspace.super_admin_id = members.user_id
			WHERE
				($2::TEXT IS NULL OR "user".email ILIKE '%' || $2::TEXT || '%') AND
				($3::TEXT IS NULL OR "user".first_name ILIKE '%' || $3::TEXT || '%') AND
				($4::TEXT IS NULL OR "user".last_name ILIKE '%' || $4::TEXT || '%')
		),
		users_page AS (
			SELECT
				*
			FROM
				matched_users
			ORDER BY
				user_id
			LIMIT $5
			OFFSET $6
		)
		SELECT
			users_page.user_id AS "user_id!",
			users_page.first_name AS "first_name!",
			users_page.last_name AS "last_name!",
			users_page.email AS "email!",
			users_page.is_owner AS "is_owner!",
			COALESCE(
				ARRAY_REMOVE(ARRAY_AGG(workspace_user.role_id), NULL),
				'{}'
			) AS "role_ids!: Vec<Uuid>",
			(SELECT COUNT(*) FROM matched_users) AS "total_count!"
		FROM
			users_page
		LEFT JOIN
			workspace_user
		ON
			workspace_user.user_id = users_page.user_id AND
			workspace_user.workspace_id = $1
		GROUP BY
			users_page.user_id,
			users_page.first_name,
			users_page.last_name,
			users_page.email,
			users_page.is_owner
		ORDER BY
			users_page.user_id;
		"#,
		workspace_id as _,
		email_filter,
		first_name_filter,
		last_name_filter,
		count as i64,
		(count * page) as i64,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		total_count = row.total_count;
		WorkspaceMember {
			user: WithId::new(
				row.user_id,
				WorkspaceUserInfo {
					first_name: row.first_name,
					last_name: row.last_name,
					email: row.email,
				},
			),
			role_ids: row.role_ids,
			is_owner: row.is_owner,
		}
	})
	.collect();

	AppResponse::builder()
		.body(ListUsersInWorkspaceResponse { users })
		.headers(ListUsersInWorkspaceResponseHeaders {
			total_count: TotalCountHeader(total_count as _),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
