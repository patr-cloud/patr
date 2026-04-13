use axum::http::StatusCode;
use models::api::workspace::{rbac::user::RoleBindingGrant, service_account::*};

use crate::prelude::*;

pub async fn get_service_account_info(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					GetServiceAccountInfoPath {
						workspace_id: _,
						service_account_id,
					},
				query: (),
				headers:
					GetServiceAccountInfoRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: GetServiceAccountInfoRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, GetServiceAccountInfoRequest>,
) -> Result<AppResponse<GetServiceAccountInfoRequest>, ErrorType> {
	let service_account = query!(
		r#"
		SELECT
			id AS "id: Uuid",
			name,
			description
		FROM
			service_account
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
		service_account_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	let role_bindings = query!(
		r#"
		SELECT
			role_id AS "role_id: Uuid",
			scope_id AS "scope_id: Uuid"
		FROM
			role_binding
		WHERE
			actor_id = $1;
		"#,
		service_account_id as _,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| RoleBindingGrant {
		role_id: row.role_id,
		resource_id: row.scope_id,
	})
	.collect();

	AppResponse::builder()
		.body(GetServiceAccountInfoResponse {
			service_account: WithId::new(
				service_account.id,
				ServiceAccount {
					name: service_account.name,
					description: service_account.description,
					role_bindings,
				},
			),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
