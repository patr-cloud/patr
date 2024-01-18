use axum::{http::StatusCode, Router};
use models::{api::workspace::rbac::role::*, ApiRequest, ErrorType};

use crate::prelude::*;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_auth_endpoint(create_new_role, state)
		.mount_auth_endpoint(delete_role, state)
		.mount_auth_endpoint(get_role_info, state)
		.mount_auth_endpoint(list_all_roles, state)
		.mount_auth_endpoint(list_users_for_roles, state)
		.mount_auth_endpoint(update_role, state)
		.with_state(state.clone())
}

async fn create_new_role(
	AuthenticatedAppRequest {
		request: ApiRequest {
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
	}: AuthenticatedAppRequest<'_, CreateNewRoleRequest>,
) -> Result<AppResponse<CreateNewRoleRequest>, ErrorType> {
	info!("Starting: Get current permissions");

	// LOGIC

	AppResponse::builder()
		.body(CreateNewRoleResponse { id: todo!() })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn delete_role(
	AuthenticatedAppRequest {
		request: ApiRequest {
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
	}: AuthenticatedAppRequest<'_, DeleteRoleRequest>,
) -> Result<AppResponse<DeleteRoleRequest>, ErrorType> {
	info!("Starting: List all permissions");

	// LOGIC

	AppResponse::builder()
		.body(DeleteRoleResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn get_role_info(
	AuthenticatedAppRequest {
		request: ApiRequest {
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
	}: AuthenticatedAppRequest<'_, GetRoleInfoRequest>,
) -> Result<AppResponse<GetRoleInfoRequest>, ErrorType> {
	info!("Starting: List all resource type");

	// LOGIC

	AppResponse::builder()
		.body(GetRoleInfoResponse {
			role: todo!(),
			resource_permissions: todo!(),
			resource_type_permissions: todo!(),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn list_all_roles(
	AuthenticatedAppRequest {
		request: ApiRequest {
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
	}: AuthenticatedAppRequest<'_, ListAllRolesRequest>,
) -> Result<AppResponse<ListAllRolesRequest>, ErrorType> {
	info!("Starting: List all resource type");

	// LOGIC

	AppResponse::builder()
		.body(ListAllRolesResponse { roles: todo!() })
		.headers(ListAllRolesResponseHeaders {
			total_count: todo!(),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn list_users_for_roles(
	AuthenticatedAppRequest {
		request: ApiRequest {
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
	}: AuthenticatedAppRequest<'_, ListUsersForRolesRequest>,
) -> Result<AppResponse<ListUsersForRolesRequest>, ErrorType> {
	info!("Starting: List all resource type");

	// LOGIC

	AppResponse::builder()
		.body(ListUsersForRolesResponse { users: todo!() })
		.headers(ListUsersForRolesResponseHeaders {
			total_count: todo!(),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn update_role(
	AuthenticatedAppRequest {
		request: ApiRequest {
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
	}: AuthenticatedAppRequest<'_, UpdateRoleRequest>,
) -> Result<AppResponse<UpdateRoleRequest>, ErrorType> {
	info!("Starting: List all resource type");

	// LOGIC

	AppResponse::builder()
		.body(UpdateRoleResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
