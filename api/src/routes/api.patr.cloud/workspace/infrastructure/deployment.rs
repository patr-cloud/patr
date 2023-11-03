use crate::prelude::*;
use axum::{http::StatusCode, Router};

use models::{
	api::workspace::infrastructure::deployment::*,
	ApiRequest,
	ErrorType,
};

#[instrument(skip(state))]
pub fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_endpoint(machine_type, state)
		.mount_auth_endpoint(list_deployment, state)
		.mount_auth_endpoint(list_deployment_history, state)
		.mount_auth_endpoint(create_deployment, state)
		.mount_auth_endpoint(get_deployment_info, state)
		.mount_auth_endpoint(start_deployment, state)
		.mount_auth_endpoint(stop_deployment, state)
		.mount_auth_endpoint(revert_deployment, state)
		.mount_auth_endpoint(get_deployment_log, state)
		.mount_auth_endpoint(delete_deployment, state)
		.mount_auth_endpoint(update_deployment, state)
		.mount_auth_endpoint(list_linked_url, state)
		.mount_auth_endpoint(get_deployment_metric, state)
		.mount_auth_endpoint(get_build_log, state)
		.mount_auth_endpoint(get_build_event, state)
		.with_state(state.clone())
}

async fn machine_type(
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
	}: AppRequest<'_, ListAllDeploymentMachineTypesRequest>,
) -> Result<AppResponse<ListAllDeploymentMachineTypesRequest>, ErrorType> {
	
	info!("Starting: List deployments");

	// LOGIC

    AppResponse::builder()
        .body(ListAllDeploymentMachineTypesResponse {
            machine_types: todo!(),
        })
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}

async fn list_deployment(
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
	}: AuthenticatedAppRequest<'_, ListDeploymentsRequest>,
) -> Result<AppResponse<ListDeploymentsRequest>, ErrorType> {
	
	info!("Starting: List deployments");

	// LOGIC

    AppResponse::builder()
        .body(ListDeploymentsResponse {
            deployments: todo!(),
        })
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}

async fn list_deployment_history(
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
	}: AuthenticatedAppRequest<'_, ListDeploymentHistoryRequest>,
) -> Result<AppResponse<ListDeploymentHistoryRequest>, ErrorType> {
	
	info!("Starting: List deployment history");

	// LOGIC

    AppResponse::builder()
        .body(ListDeploymentHistoryResponse {
            deploys: todo!(),
        })
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}

async fn create_deployment(
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
	}: AuthenticatedAppRequest<'_, CreateDeploymentRequest>,
) -> Result<AppResponse<CreateDeploymentRequest>, ErrorType> {
	
	info!("Starting: Create deployment");

	// LOGIC

    AppResponse::builder()
        .body(CreateDeploymentResponse {
            id: todo!(),
        })
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}

async fn get_deployment_info(
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
	}: AuthenticatedAppRequest<'_, GetDeploymentInfoRequest>,
) -> Result<AppResponse<GetDeploymentInfoRequest>, ErrorType> {
	
	info!("Starting: Get deployment info");

	// LOGIC

    AppResponse::builder()
        .body(GetDeploymentInfoResponse {
            deployment: todo!(),
            running_details: todo!()
        })
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}

async fn start_deployment(
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
	}: AuthenticatedAppRequest<'_, StartDeploymentRequest>,
) -> Result<AppResponse<StartDeploymentRequest>, ErrorType> {
	
	info!("Starting: Start deployment");

	// LOGIC

    AppResponse::builder()
        .body(StartDeploymentResponse)
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}

async fn stop_deployment(
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
	}: AuthenticatedAppRequest<'_, StopDeploymentRequest>,
) -> Result<AppResponse<StopDeploymentRequest>, ErrorType> {
	
	info!("Starting: Stop deployment");

	// LOGIC

    AppResponse::builder()
        .body(StopDeploymentResponse)
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}

async fn revert_deployment(
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
	}: AuthenticatedAppRequest<'_, RevertDeploymentRequest>,
) -> Result<AppResponse<RevertDeploymentRequest>, ErrorType> {
	
	info!("Starting: Revert deployment");

	// LOGIC

    AppResponse::builder()
        .body(RevertDeploymentResponse)
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}

async fn get_deployment_log(
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
	}: AuthenticatedAppRequest<'_, GetDeploymentLogsRequest>,
) -> Result<AppResponse<GetDeploymentLogsRequest>, ErrorType> {
	
	info!("Starting: Get deployment logs");

	// LOGIC

    AppResponse::builder()
        .body(GetDeploymentLogsResponse {
            logs: todo!(),
        })
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}

async fn delete_deployment(
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
	}: AuthenticatedAppRequest<'_, DeleteDeploymentRequest>,
) -> Result<AppResponse<DeleteDeploymentRequest>, ErrorType> {
	
	info!("Starting: Delete deployment");

	// LOGIC

    AppResponse::builder()
        .body(DeleteDeploymentResponse)
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}

async fn update_deployment(
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
	}: AuthenticatedAppRequest<'_, UpdateDeploymentRequest>,
) -> Result<AppResponse<UpdateDeploymentRequest>, ErrorType> {
	
	info!("Starting: List linked URLs");

	// LOGIC

    AppResponse::builder()
        .body(UpdateDeploymentResponse)
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}

async fn list_linked_url(
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
	}: AuthenticatedAppRequest<'_, ListLinkedURLsRequest>,
) -> Result<AppResponse<ListLinkedURLsRequest>, ErrorType> {
	
	info!("Starting: List linked URLs");

	// LOGIC

    AppResponse::builder()
        .body(ListLinkedURLsResponse {
            urls: todo!(),
        })
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}

async fn get_deployment_metric(
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
	}: AuthenticatedAppRequest<'_, GetDeploymentMetricsRequest>,
) -> Result<AppResponse<GetDeploymentMetricsRequest>, ErrorType> {
	
	info!("Starting: Get deployment metrics");

	// LOGIC

    AppResponse::builder()
        .body(GetDeploymentMetricsResponse {
            metrics: todo!(),
        })
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}

async fn get_build_log(
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
	}: AuthenticatedAppRequest<'_, GetDeploymentBuildLogsRequest>,
) -> Result<AppResponse<GetDeploymentBuildLogsRequest>, ErrorType> {
	
	info!("Starting: Get deployment build logs");

	// LOGIC

    AppResponse::builder()
        .body(GetDeploymentBuildLogsResponse {
            logs: todo!(),
        })
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}

async fn get_build_event(
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
	}: AuthenticatedAppRequest<'_, GetDeploymentEventsRequest>,
) -> Result<AppResponse<GetDeploymentEventsRequest>, ErrorType> {
	
	info!("Starting: Get deployment build events");

	// LOGIC

    AppResponse::builder()
        .body(GetDeploymentEventsResponse {
            logs: todo!(),
        })
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}
