use axum::http::StatusCode;
use models::api::workspace::rbac::*;

use crate::prelude::*;

pub async fn list_all_permissions(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					ListAllPermissionsPath {
						// For now we're ignoring it. Maybe in the future we
						// might have different permissions based on workspace
						workspace_id: _,
					},
				query: (),
				headers:
					ListAllPermissionsRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ListAllPermissionsRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, ListAllPermissionsRequest>,
) -> Result<AppResponse<ListAllPermissionsRequest>, ErrorType> {
	info!("Listing all permissions in the database");

	let permissions = query!(
		r#"
		SELECT
			*
		FROM
			permission;
		"#
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		WithId::new(
			row.id,
			Permission {
				name: row.name,
				description: row.description,
			},
		)
	})
	.collect();

	AppResponse::builder()
		.body(ListAllPermissionsResponse { permissions })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
