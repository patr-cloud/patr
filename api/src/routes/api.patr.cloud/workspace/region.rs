use axum::{http::StatusCode, Router};
use models::{api::workspace::region::*, ApiRequest, ErrorType};

use crate::prelude::*;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_auth_endpoint(add_region_to_workspace, state)
		.mount_auth_endpoint(check_region_status, state)
		.mount_auth_endpoint(delete_region, state)
		.mount_auth_endpoint(get_region_info, state)
		.mount_auth_endpoint(list_regions_for_workspace, state)
		.with_state(state.clone())
}

async fn add_region_to_workspace(
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
	}: AuthenticatedAppRequest<'_, AddRegionToWorkspaceRequest>,
) -> Result<AppResponse<AddRegionToWorkspaceRequest>, ErrorType> {
	info!("Starting: Add region to a workspace");

	// LOGIC

	AppResponse::builder()
		.body(AddRegionToWorkspaceResponse { region_id: todo!() })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn check_region_status(
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
	}: AuthenticatedAppRequest<'_, CheckRegionStatusRequest>,
) -> Result<AppResponse<CheckRegionStatusRequest>, ErrorType> {
	info!("Starting: Check region status");

	// LOGIC

	AppResponse::builder()
		.body(CheckRegionStatusResponse {
			region: todo!(),
			message_log: todo!(),
			disconnected_at: todo!(),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn delete_region(
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
	}: AuthenticatedAppRequest<'_, DeleteRegionRequest>,
) -> Result<AppResponse<DeleteRegionRequest>, ErrorType> {
	info!("Starting: Delete region");

	// LOGIC

	AppResponse::builder()
		.body(DeleteRegionResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn get_region_info(
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
	}: AuthenticatedAppRequest<'_, GetRegionInfoRequest>,
) -> Result<AppResponse<GetRegionInfoRequest>, ErrorType> {
	info!("Starting: Get region info");

	// LOGIC

	AppResponse::builder()
		.body(GetRegionInfoResponse {
			region: todo!(),
			message_log: todo!(),
			disconnected_at: todo!(),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn list_regions_for_workspace(
	AuthenticatedAppRequest {
		request:
			ApiRequest {
				path: ListRegionsForWorkspacePath { workspace_id },
				query: Paginated {
					data: (),
					count,
					page,
				},
				headers: _,
				body: ListRegionsForWorkspaceRequest,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, ListRegionsForWorkspaceRequest>,
) -> Result<AppResponse<ListRegionsForWorkspaceRequest>, ErrorType> {
	info!("Starting: List region for workspace");

	// LOGIC

	AppResponse::builder()
		.body(ListRegionsForWorkspaceResponse { regions: todo!() })
		.headers(ListRegionsForWorkspaceResponseHeaders {
			total_count: todo!(),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
