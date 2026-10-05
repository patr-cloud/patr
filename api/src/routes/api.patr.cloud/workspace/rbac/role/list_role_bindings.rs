use axum::http::StatusCode;
use models::api::{
	WithId,
	workspace::{
		rbac::{role::*, user::WorkspaceUserInfo},
		service_account::ServiceAccount,
	},
};

use crate::prelude::*;

/// The handler to list every binding of a role in the workspace: each member or
/// service account holding it, and the resource it applies at.
pub async fn list_role_bindings(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListRoleBindingsPath {
					workspace_id,
					role_id,
				},
				query: (),
				headers:
					ListRoleBindingsRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ListRoleBindingsRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		actor_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, ListRoleBindingsRequest>,
) -> Result<AppResponse<ListRoleBindingsRequest>, ErrorType> {
	info!("Listing the bindings of role: {}", role_id);

	query!(
		r#"
		SELECT
			id
		FROM
			role
		WHERE
			id = $1 AND
			workspace_id = $2;
		"#,
		role_id as _,
		workspace_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::RoleDoesNotExist)?;

	let bindings = query!(
		r#"
		SELECT
			role_binding.actor_id AS "actor_id: Uuid",
			role_binding.scope_id AS "scope_id: Uuid",
			workspace_actor.actor_type AS "actor_type: WorkspaceActorDiscriminant",
			workspace_user.user_id AS "user_id?: Uuid",
			"user".first_name AS "first_name?",
			"user".last_name AS "last_name?",
			"user".email AS "email?",
			service_account.name AS "service_account_name?",
			service_account.description AS "service_account_description?"
		FROM
			role_binding
		INNER JOIN
			workspace_actor
		ON
			workspace_actor.id = role_binding.actor_id
		LEFT JOIN
			workspace_user
		ON
			workspace_user.actor_id = role_binding.actor_id
		LEFT JOIN
			"user"
		ON
			"user".id = workspace_user.user_id
		LEFT JOIN
			service_account
		ON
			service_account.id = role_binding.actor_id AND
			service_account.deleted IS NULL
		WHERE
			role_binding.workspace_id = $1 AND
			role_binding.role_id = $2
		ORDER BY
			role_binding.created;
		"#,
		workspace_id as _,
		role_id as _,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		let actor = match row.actor_type {
			WorkspaceActorDiscriminant::User => WithId::new(
				row.user_id
					.ok_or_else(|| ErrorType::server_error("user_id in db is NULL"))?,
				WorkspaceActor::User(WorkspaceUserInfo {
					first_name: row
						.first_name
						.ok_or_else(|| ErrorType::server_error("first_name in db is NULL"))?,
					last_name: row
						.last_name
						.ok_or_else(|| ErrorType::server_error("last_name in db is NULL"))?,
					email: row
						.email
						.ok_or_else(|| ErrorType::server_error("email in db is NULL"))?,
				}),
			),
			WorkspaceActorDiscriminant::ServiceAccount => WithId::new(
				row.actor_id,
				WorkspaceActor::ServiceAccount(ServiceAccount {
					name: row.service_account_name.ok_or_else(|| {
						ErrorType::server_error("service account name in db is NULL")
					})?,
					description: row.service_account_description,
				}),
			),
		};

		Ok(RoleBinding {
			actor,
			resource_id: row.scope_id,
		})
	})
	.collect::<Result<Vec<_>, ErrorType>>()?;

	AppResponse::builder()
		.body(ListRoleBindingsResponse { bindings })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
