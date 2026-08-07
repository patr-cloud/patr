use axum::http::StatusCode;
use models::api::workspace::service_account::*;

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

	let roles = query!(
		r#"
		SELECT
			role_id AS "role_id: Uuid"
		FROM
			workspace_member
		WHERE
			identity_id = $1;
		"#,
		service_account_id as _,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|r| r.role_id)
	.collect();

	AppResponse::builder()
		.body(GetServiceAccountInfoResponse {
			service_account: WithId::new(
				service_account.id,
				ServiceAccount {
					name: service_account.name,
					description: service_account.description,
					roles,
				},
			),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
