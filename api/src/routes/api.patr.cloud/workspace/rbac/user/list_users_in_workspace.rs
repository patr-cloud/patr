use std::collections::BTreeMap;

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
	let rows = query!(
		r#"
		WITH members AS (
			SELECT
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
		)
		SELECT
			members.user_id AS "user_id!: Uuid",
			"user".first_name AS "first_name!",
			"user".last_name AS "last_name!",
			"user".email AS "email!",
			(workspace.id IS NOT NULL) AS "is_owner!",
			COUNT(*) OVER() AS "total_count!"
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
		ORDER BY
			members.user_id
		LIMIT $5
		OFFSET $6;
		"#,
		workspace_id as _,
		email_filter,
		first_name_filter,
		last_name_filter,
		count as i64,
		(count * page) as i64,
	)
	.fetch_all(&mut **database)
	.await?;

	// One flat query for the whole page's grants, then attach. Keeping this
	// out of the query above means the page is paginated over users, not over
	// the user-by-binding fan-out.
	let mut grants = query!(
		r#"
		SELECT
			workspace_user.user_id AS "user_id: Uuid",
			role_binding.role_id AS "role_id: Uuid",
			role_binding.scope_id AS "scope_id: Uuid"
		FROM
			role_binding
		INNER JOIN
			workspace_user
		ON
			role_binding.actor_id = workspace_user.actor_id
		WHERE
			workspace_user.workspace_id = $1 AND
			workspace_user.user_id = ANY($2::UUID[]);
		"#,
		workspace_id as _,
		&rows.iter().map(|member| member.user_id).collect::<Vec<_>>() as _,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.fold(
		BTreeMap::<Uuid, Vec<RoleBindingGrant>>::new(),
		|mut grants, row| {
			grants
				.entry(row.user_id.into())
				.or_default()
				.push(RoleBindingGrant {
					role_id: row.role_id.into(),
					resource_id: row.scope_id.into(),
				});
			grants
		},
	);

	let users = rows
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
				role_bindings: grants.remove(&row.user_id).unwrap_or_default(),
				is_owner: row.is_owner,
			}
		})
		.collect::<Vec<_>>();

	AppResponse::builder()
		.body(ListUsersInWorkspaceResponse { users })
		.headers(ListUsersInWorkspaceResponseHeaders {
			total_count: TotalCountHeader(total_count as _),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
