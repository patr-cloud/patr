use axum::{http::StatusCode, Router};
use models::api::workspace::domain::*;

use crate::prelude::*;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_endpoint(is_domain_personal, state)
		.mount_auth_endpoint(add_dns_record, state)
		.mount_auth_endpoint(add_domain_to_workspace, state)
		.mount_auth_endpoint(delete_dns_record, state)
		.mount_auth_endpoint(delete_domain_in_workspace, state)
		.mount_auth_endpoint(get_doamin_dns_record, state)
		.mount_auth_endpoint(get_domain_info_in_workspace, state)
		.mount_auth_endpoint(get_domains_for_workspace, state)
		.mount_auth_endpoint(update_domain_dns_record, state)
		.mount_auth_endpoint(verify_domain_in_workspace, state)
}

async fn is_domain_personal(
	AppRequest {
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
	}: AppRequest<'_, IsDomainPersonalRequest>,
) -> Result<AppResponse<IsDomainPersonalRequest>, ErrorType> {
	info!("Starting: Check for is domain personal");

	// LOGIC

	AppResponse::builder()
		.body(IsDomainPersonalResponse {
			personal: todo!(),
			is_used_by_others: todo!(),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn add_dns_record(
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
	}: AuthenticatedAppRequest<'_, AddDNSRecordRequest>,
) -> Result<AppResponse<AddDNSRecordRequest>, ErrorType> {
	info!("Starting: Add DNS record");

	// LOGIC

	AppResponse::builder()
		.body(AddDNSRecordResponse { id: todo!() })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn add_domain_to_workspace(
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
	}: AuthenticatedAppRequest<'_, AddDomainToWorkspaceRequest>,
) -> Result<AppResponse<AddDomainToWorkspaceRequest>, ErrorType> {
	info!("Starting: Add domain to workspace");

	// LOGIC

	AppResponse::builder()
		.body(AddDomainToWorkspaceResponse { id: todo!() })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn delete_dns_record(
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
	}: AuthenticatedAppRequest<'_, DeleteDNSRecordRequest>,
) -> Result<AppResponse<DeleteDNSRecordRequest>, ErrorType> {
	info!("Starting: Delete DNS record");

	// LOGIC

	AppResponse::builder()
		.body(DeleteDNSRecordResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn delete_domain_in_workspace(
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
	}: AuthenticatedAppRequest<'_, DeleteDomainInWorkspaceRequest>,
) -> Result<AppResponse<DeleteDomainInWorkspaceRequest>, ErrorType> {
	info!("Starting: Delete domain in workspace");

	// LOGIC

	AppResponse::builder()
		.body(DeleteDomainInWorkspaceResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn get_doamin_dns_record(
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
	}: AuthenticatedAppRequest<'_, GetDomainDNSRecordRequest>,
) -> Result<AppResponse<GetDomainDNSRecordRequest>, ErrorType> {
	info!("Starting: Get domain DNS record");

	// LOGIC

	AppResponse::builder()
		.body(GetDomainDNSRecordResponse { records: todo!() })
		.headers(GetDomainDNSRecordResponseHeaders {
			total_count: todo!(),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn get_domain_info_in_workspace(
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
	}: AuthenticatedAppRequest<'_, GetDomainInfoInWorkspaceRequest>,
) -> Result<AppResponse<GetDomainInfoInWorkspaceRequest>, ErrorType> {
	info!("Starting: Get domain info in workspace");

	// LOGIC

	AppResponse::builder()
		.body(GetDomainInfoInWorkspaceResponse {
			workspace_domain: todo!(),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn get_domains_for_workspace(
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
	}: AuthenticatedAppRequest<'_, GetDomainsForWorkspaceRequest>,
) -> Result<AppResponse<GetDomainsForWorkspaceRequest>, ErrorType> {
	info!("Starting: Get domains for workspace");

	// LOGIC

	AppResponse::builder()
		.body(GetDomainsForWorkspaceResponse { domains: todo!() })
		.headers(GetDomainsForWorkspaceResponseHeaders {
			total_count: todo!(),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn update_domain_dns_record(
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
	}: AuthenticatedAppRequest<'_, UpdateDomainDNSRecordRequest>,
) -> Result<AppResponse<UpdateDomainDNSRecordRequest>, ErrorType> {
	info!("Starting: Update domain DNS record");

	// LOGIC

	AppResponse::builder()
		.body(UpdateDomainDNSRecordResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn verify_domain_in_workspace(
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
	}: AuthenticatedAppRequest<'_, VerifyDomainInWorkspaceRequest>,
) -> Result<AppResponse<VerifyDomainInWorkspaceRequest>, ErrorType> {
	info!("Starting: Check to verify domain in workspace");

	// LOGIC

	AppResponse::builder()
		.body(VerifyDomainInWorkspaceResponse { verified: todo!() })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
