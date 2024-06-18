use axum::http::StatusCode;
use models::api::workspace::rbac::*;

use crate::prelude::*;

/// The handler to list all resource types in the database. This will return all
/// resource types that are available to a user. This will not return the
/// resource types of the user, but all resource types that are available in the
/// database. This is useful for the user to know what resource types are
/// available to them.
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
