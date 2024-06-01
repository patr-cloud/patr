use axum::http::StatusCode;
use models::api::workspace::rbac::*;

use crate::prelude::*;

pub async fn list_all_resource_types(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					ListAllResourceTypesPath {
						// For now we're ignoring it. Maybe in the future we
						// might have different resource types based on workspace
						workspace_id: _,
					},
				query: (),
				headers:
					ListAllResourceTypesRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ListAllResourceTypesRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, ListAllResourceTypesRequest>,
) -> Result<AppResponse<ListAllResourceTypesRequest>, ErrorType> {
	info!("Listing all resource types in the database");

	let resource_types = query!(
		r#"
		SELECT
			*
		FROM
			resource_type;
		"#
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		WithId::new(
			row.id,
			ResourceType {
				name: row.name,
				description: row.description,
			},
		)
	})
	.collect();

	AppResponse::builder()
		.body(ListAllResourceTypesResponse { resource_types })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
