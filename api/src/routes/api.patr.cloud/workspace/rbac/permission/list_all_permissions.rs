use axum::http::StatusCode;
use models::api::workspace::rbac::{Permission as PermissionModel, *};

use crate::prelude::*;

/// The handler to list all permissions in the database. This will return all
/// permissions that are available to a user. This will not return the
/// permissions of the user, but all permissions that are available in the
/// database. This is useful for the user to know what permissions are available
/// to them.
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
			PermissionModel {
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
