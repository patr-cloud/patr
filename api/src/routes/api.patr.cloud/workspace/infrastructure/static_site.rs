use crate::{prelude::*, service};
use axum::{http::StatusCode, Router};

use models::{
	api::workspace::infrastructure::static_site::*,
	ApiRequest,
	ErrorType, prelude::WithId,
};

#[instrument(skip(state))]
pub fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_endpoint(create_static_site, state)
		.mount_endpoint(delete_static_site, state)
		.mount_endpoint(get_static_site_info, state)
		.mount_endpoint(list_static_site, state)
		.mount_endpoint(list_upload_history, state)
		.mount_endpoint(revert_static_site, state)
		.mount_endpoint(start_static_site, state)
		.mount_endpoint(stop_static_site, state)
		.mount_endpoint(update_static_site, state)
		.mount_endpoint(upload_static_site, state)
		.mount_endpoint(list_linked_url, state)
		.with_state(state.clone())
}

async fn create_static_site(
	AppRequest {
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
	}: AppRequest<'_, CreateStaticSiteRequest>,
) -> Result<AppResponse<CreateStaticSiteResponse>, ErrorType> {
	
	info!("Starting: Create static site");

	// LOGIC

    AppResponse::builder()
        .body(CreateStaticSiteResponse {
            id: todo!(),
        })
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}

async fn delete_static_site(
	AppRequest {
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
	}: AppRequest<'_, DeleteStaticSiteRequest>,
) -> Result<AppResponse<DeleteStaticSiteResponse>, ErrorType> {
	
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
	AppRequest {
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
	}: AppRequest<'_, GetStaticSiteInfoRequest>,
) -> Result<AppResponse<GetStaticSiteInfoResponse>, ErrorType> {
	
	info!("Starting: Get static site info");

	// LOGIC

    AppResponse::builder()
        .body(GetStaticSiteInfoResponse {
            static_site: todo!(),
            static_site_details: todo!()
        })
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}

async fn list_static_site(
	AppRequest {
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
	}: AppRequest<'_, ListStaticSiteRequest>,
) -> Result<AppResponse<ListStaticSiteResponse>, ErrorType> {
	
	info!("Starting: List static site");

	// LOGIC

    AppResponse::builder()
        .body(ListStaticSiteResponse {
            static_sites: todo!(),
        })
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}

async fn list_upload_history(
	AppRequest {
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
	}: AppRequest<'_, ListStaticSiteUploadHistoryRequest>,
) -> Result<AppResponse<ListStaticSiteUploadHistoryResponse>, ErrorType> {
	
	info!("Starting: List static site upload history");

	// LOGIC

    AppResponse::builder()
        .body(ListStaticSiteUploadHistoryResponse {
            uploads: todo!(),
        })
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}

async fn revert_static_site(
	AppRequest {
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
	}: AppRequest<'_, RevertStaticSiteRequest>,
) -> Result<AppResponse<RevertStaticSiteResponse>, ErrorType> {
	
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
	AppRequest {
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
	}: AppRequest<'_, StartStaticSiteRequest>,
) -> Result<AppResponse<StartStaticSiteResponse>, ErrorType> {
	
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
	AppRequest {
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
	}: AppRequest<'_, StopStaticSiteRequest>,
) -> Result<AppResponse<StopStaticSiteResponse>, ErrorType> {
	
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
	AppRequest {
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
	}: AppRequest<'_, UpdateStaticSiteRequest>,
) -> Result<AppResponse<UpdateStaticSiteResponse>, ErrorType> {
	
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
	AppRequest {
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
	}: AppRequest<'_, UploadStaticSiteRequest>,
) -> Result<AppResponse<UploadStaticSiteResponse>, ErrorType> {
	
	info!("Starting: Upload static site");

	// LOGIC

    AppResponse::builder()
        .body(UploadStaticSiteResponse {
            upload_id: todo!(),
        })
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}

async fn list_linked_url(
	AppRequest {
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
	}: AppRequest<'_, ListLinkedUrlRequest>,
) -> Result<AppResponse<ListLinkedUrlResponse>, ErrorType> {
	
	info!("Starting: List linked URL");

	// LOGIC

    AppResponse::builder()
        .body(ListLinkedUrlResponse {
            urls: todo!(),
        })
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}