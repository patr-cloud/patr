use axum::{http::StatusCode, Router};
use models::api::workspace::static_site::*;

use crate::prelude::*;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_auth_endpoint(create_static_site, state)
		.mount_auth_endpoint(delete_static_site, state)
		.mount_auth_endpoint(get_static_site_info, state)
		.mount_auth_endpoint(list_static_site, state)
		.mount_auth_endpoint(list_upload_history, state)
		.mount_auth_endpoint(revert_static_site, state)
		.mount_auth_endpoint(start_static_site, state)
		.mount_auth_endpoint(stop_static_site, state)
		.mount_auth_endpoint(update_static_site, state)
		.mount_auth_endpoint(upload_static_site, state)
}

async fn create_static_site(
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
	}: AuthenticatedAppRequest<'_, CreateStaticSiteRequest>,
) -> Result<AppResponse<CreateStaticSiteRequest>, ErrorType> {
	info!("Starting: Create static site");

	// LOGIC

	AppResponse::builder()
		.body(CreateStaticSiteResponse { id: todo!() })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn delete_static_site(
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
	}: AuthenticatedAppRequest<'_, DeleteStaticSiteRequest>,
) -> Result<AppResponse<DeleteStaticSiteRequest>, ErrorType> {
	info!("Starting: Delete static site");

	// LOGIC

	AppResponse::builder()
		.body(DeleteStaticSiteResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn get_static_site_info(
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
	}: AuthenticatedAppRequest<'_, GetStaticSiteInfoRequest>,
) -> Result<AppResponse<GetStaticSiteInfoRequest>, ErrorType> {
	info!("Starting: Get static site info");

	// LOGIC

	AppResponse::builder()
		.body(GetStaticSiteInfoResponse {
			static_site: todo!(),
			static_site_details: todo!(),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn list_static_site(
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
	}: AuthenticatedAppRequest<'_, ListStaticSiteRequest>,
) -> Result<AppResponse<ListStaticSiteRequest>, ErrorType> {
	info!("Starting: List static site");

	// LOGIC

	AppResponse::builder()
		.body(ListStaticSiteResponse {
			static_sites: todo!(),
		})
		.headers(ListStaticSiteResponseHeaders {
			total_count: todo!(),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn list_upload_history(
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
	}: AuthenticatedAppRequest<'_, ListStaticSiteUploadHistoryRequest>,
) -> Result<AppResponse<ListStaticSiteUploadHistoryRequest>, ErrorType> {
	info!("Starting: List static site upload history");

	// LOGIC

	AppResponse::builder()
		.body(ListStaticSiteUploadHistoryResponse { uploads: todo!() })
		.headers(ListStaticSiteUploadHistoryResponseHeaders {
			total_count: todo!(),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn revert_static_site(
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
	}: AuthenticatedAppRequest<'_, RevertStaticSiteRequest>,
) -> Result<AppResponse<RevertStaticSiteRequest>, ErrorType> {
	info!("Starting: Revert static site");

	// LOGIC

	AppResponse::builder()
		.body(RevertStaticSiteResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn start_static_site(
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
	}: AuthenticatedAppRequest<'_, StartStaticSiteRequest>,
) -> Result<AppResponse<StartStaticSiteRequest>, ErrorType> {
	info!("Starting: Start static site");

	// LOGIC

	AppResponse::builder()
		.body(StartStaticSiteResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn stop_static_site(
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
	}: AuthenticatedAppRequest<'_, StopStaticSiteRequest>,
) -> Result<AppResponse<StopStaticSiteRequest>, ErrorType> {
	info!("Starting: Stop static site");

	// LOGIC

	AppResponse::builder()
		.body(StopStaticSiteResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn update_static_site(
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
	}: AuthenticatedAppRequest<'_, UpdateStaticSiteRequest>,
) -> Result<AppResponse<UpdateStaticSiteRequest>, ErrorType> {
	info!("Starting: Update static site");

	// LOGIC

	AppResponse::builder()
		.body(UpdateStaticSiteResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn upload_static_site(
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
	}: AuthenticatedAppRequest<'_, UploadStaticSiteRequest>,
) -> Result<AppResponse<UploadStaticSiteRequest>, ErrorType> {
	info!("Starting: Upload static site");

	// LOGIC

	AppResponse::builder()
		.body(UploadStaticSiteResponse { upload_id: todo!() })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
