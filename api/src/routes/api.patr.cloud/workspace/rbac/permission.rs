use axum::{http::StatusCode, Router};
use models::api::workspace::rbac::*;

use crate::prelude::*;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_auth_endpoint(get_current_permissions, state)
		.mount_auth_endpoint(list_all_permissions, state)
		.mount_auth_endpoint(list_all_resource_types, state)
		.with_state(state.clone())
}

async fn get_current_permissions(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetCurrentPermissionsPath { workspace_id },
				query: (),
				headers,
				body,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, GetCurrentPermissionsRequest>,
) -> Result<AppResponse<GetCurrentPermissionsRequest>, ErrorType> {
	info!("Get permissions of current request");

	AppResponse::builder()
		.body(GetCurrentPermissionsResponse {
			permissions: user_data
				.permissions
				.get(&workspace_id)
				.ok_or_else(|| ErrorType::server_error("workspace not found in user_data"))?
				.clone(),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn list_all_permissions(
	AuthenticatedAppRequest {
		request: ProcessedApiRequest {
			path,
			query: _,
			headers,
			body,
		},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, ListAllPermissionsRequest>,
) -> Result<AppResponse<ListAllPermissionsRequest>, ErrorType> {
	info!("Starting: List all permissions");

	// LOGIC

	AppResponse::builder()
		.body(ListAllPermissionsResponse {
			permissions: todo!(),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn list_all_resource_types(
	AuthenticatedAppRequest {
		request: ProcessedApiRequest {
			path,
			query: _,
			headers,
			body,
		},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, ListAllResourceTypesRequest>,
) -> Result<AppResponse<ListAllResourceTypesRequest>, ErrorType> {
	info!("Starting: List all resource type");

	// LOGIC

	AppResponse::builder()
		.body(ListAllResourceTypesResponse {
			resource_types: todo!(),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
