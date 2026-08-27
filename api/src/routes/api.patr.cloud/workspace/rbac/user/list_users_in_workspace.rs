use std::collections::BTreeSet;

use axum::http::StatusCode;
use models::{
	api::{
		WithId,
		workspace::rbac::user::{WorkspaceUserInfo, WorkspaceUserInfoSearchParams, *},
	},
	rbac::PermissionScope,
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
			role_binding.role_id AS "role_id?",
			(role_binding.scope_id = role_binding.workspace_id) AS "is_workspace_scope?",
			role_binding.scope_id AS "scope_id?",
			(SELECT COUNT(*) FROM matched_users) AS "total_count!"
		FROM
			users_page
		LEFT JOIN
			workspace_user
		ON
			workspace_user.user_id = users_page.user_id AND
			workspace_user.workspace_id = $1
		LEFT JOIN
			role_binding
		ON
			role_binding.actor_id = workspace_user.actor_id
		ORDER BY
			users_page.user_id,
			role_binding.role_id;
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
	.fold(Vec::<WorkspaceMember>::new(), |mut users, row| {
		total_count = row.total_count;
		// Rows arrive ordered by user, so a member's bindings are contiguous
		// and the member being filled is always the last one pushed.
		if users
			.last()
			.is_none_or(|member| member.user.id != row.user_id.into())
		{
			users.push(WorkspaceMember {
				user: WithId::new(
					row.user_id,
					WorkspaceUserInfo {
						first_name: row.first_name,
						last_name: row.last_name,
						email: row.email,
					},
				),
				roles: Vec::new(),
				is_owner: row.is_owner,
			});
		}
		let member = users.last_mut().expect("pushed above if it was missing");

		// A zero-binding member still gets an entry, with no grants; per-
		// resource bindings of one role accumulate into a single grant.
		let (Some(role_id), Some(is_workspace_scope), Some(scope_id)) =
			(row.role_id, row.is_workspace_scope, row.scope_id)
		else {
			return users;
		};
		let role_id = role_id.into();

		let scope = if is_workspace_scope {
			PermissionScope::Workspace
		} else {
			PermissionScope::Resources(BTreeSet::from([scope_id.into()]))
		};

		if let Some(grant) = member
			.roles
			.iter_mut()
			.find(|grant| grant.role_id == role_id)
		{
			grant.scope.union_with(&scope);
		} else {
			member.roles.push(RoleGrant { role_id, scope });
		}

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
