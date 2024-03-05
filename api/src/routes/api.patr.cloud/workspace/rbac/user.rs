use axum::{http::StatusCode, Router};
use models::{api::workspace::rbac::user::*, ApiRequest, ErrorType};

use crate::prelude::*;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_auth_endpoint(list_users_with_roles_in_workspace, state)
		.mount_auth_endpoint(remove_user_from_workspace, state)
		.mount_auth_endpoint(update_user_roles_in_workspace, state)
		.with_state(state.clone())
}

async fn list_users_with_roles_in_workspace(
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
	}: AuthenticatedAppRequest<'_, ListUsersWithRolesInWorkspaceRequest>,
) -> Result<AppResponse<ListUsersWithRolesInWorkspaceRequest>, ErrorType> {
	info!("Starting: List all permissions");

	// LOGIC

	AppResponse::builder()
		.body(ListUsersWithRolesInWorkspaceResponse { users: todo!() })
		.headers(ListUsersWithRolesInWorkspaceResponseHeaders {
			total_count: todo!(),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn remove_user_from_workspace(
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
	}: AuthenticatedAppRequest<'_, RemoveUserFromWorkspaceRequest>,
) -> Result<AppResponse<RemoveUserFromWorkspaceRequest>, ErrorType> {
	info!("Starting: List all resource type");

	// LOGIC

	AppResponse::builder()
		.body(RemoveUserFromWorkspaceResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn update_user_roles_in_workspace(
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
	}: AuthenticatedAppRequest<'_, UpdateUserRolesInWorkspaceRequest>,
) -> Result<AppResponse<UpdateUserRolesInWorkspaceRequest>, ErrorType> {
	info!("Starting: List all resource type");

	// LOGIC

	AppResponse::builder()
		.body(UpdateUserRolesInWorkspaceResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
