use crate::prelude::*;
use axum::{http::StatusCode, Router};

use models::{
	api::workspace::infrastructure::deployment::*,
	ApiRequest,
	ErrorType, prelude::WithId,
};

#[instrument(skip(state))]
pub fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_endpoint(machine_type, state)
		.mount_endpoint(list_deployments, state)
		.mount_endpoint(list_deployment_history, state)
		.mount_endpoint(create_deployment, state)
		.mount_endpoint(get_deployment_info, state)
		.mount_endpoint(start_deployment, state)
		.mount_endpoint(stop_deployment, state)
		.mount_endpoint(revert_deployment, state)
		.mount_endpoint(get_deployment_logs, state)
		.mount_endpoint(delete_deployment, state)
		.mount_endpoint(update_deployment, state)
		.mount_endpoint(list_linked_urls, state)
		.mount_endpoint(get_deployment_metrics, state)
		.mount_endpoint(get_build_logs, state)
		.mount_endpoint(get_build_events, state)
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
) -> Result<AppResponse<ListAllDeploymentMachineTypesResponse>, ErrorType> {
	
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

async fn list_deployments(
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
	}: AppRequest<'_, ListDeploymentsRequest>,
) -> Result<AppResponse<ListDeploymentsResponse>, ErrorType> {
	
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
	}: AppRequest<'_, ListDeploymentHistoryRequest>,
) -> Result<AppResponse<ListDeploymentHistoryResponse>, ErrorType> {
	
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
	}: AppRequest<'_, CreateDeploymentRequest>,
) -> Result<AppResponse<CreateDeploymentResponse>, ErrorType> {
	
	info!("Starting: Create deployment");

	// LOGIC

    AppResponse::builder()
        .body(CreateDatabaseResponse {
            id: todo!(),
        })
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}

async fn get_deployment_info(
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
	}: AppRequest<'_, GetDeploymentInfoRequest>,
) -> Result<AppResponse<GetDeploymentInfoResponse>, ErrorType> {
	
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
	}: AppRequest<'_, StartDeploymentRequest>,
) -> Result<AppResponse<StartDeploymentResponse>, ErrorType> {
	
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
	}: AppRequest<'_, StopDeploymentRequest>,
) -> Result<AppResponse<StopDeploymentResponse>, ErrorType> {
	
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
	}: AppRequest<'_, RevertDeploymentRequest>,
) -> Result<AppResponse<RevertDeploymentResponse>, ErrorType> {
	
	info!("Starting: Revert deployment");

	// LOGIC

    AppResponse::builder()
        .body(RevertDeploymentResponse)
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}

async fn get_deployment_logs(
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
	}: AppRequest<'_, GetDeploymentLogsRequest>,
) -> Result<AppResponse<GetDeploymentLogsResponse>, ErrorType> {
	
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
	}: AppRequest<'_, DeleteDeploymentRequest>,
) -> Result<AppResponse<DeleteDeploymentResponse>, ErrorType> {
	
	info!("Starting: Delete deployment");

	// LOGIC

    AppResponse::builder()
        .body(DeleteDeploymentResponse)
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}

async fn list_linked_urls(
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
	}: AppRequest<'_, ListLinkedURLsRequest>,
) -> Result<AppResponse<ListLinkedURLsResponse>, ErrorType> {
	
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

async fn get_deployment_metrics(
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
	}: AppRequest<'_, GetDeploymentMetricsRequest>,
) -> Result<AppResponse<GetDeploymentMetricsResponse>, ErrorType> {
	
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

async fn get_build_logs(
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
	}: AppRequest<'_, GetDeploymentBuildLogsRequest>,
) -> Result<AppResponse<GetDeploymentBuildLogsResponse>, ErrorType> {
	
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

async fn get_build_events(
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
	}: AppRequest<'_, GetDeploymentEventsRequest>,
) -> Result<AppResponse<GetDeploymentEventsResponse>, ErrorType> {
	
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
