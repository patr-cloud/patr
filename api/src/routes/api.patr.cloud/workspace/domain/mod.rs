use axum::{Router, http::StatusCode};
use models::api::workspace::domain::*;

use crate::prelude::*;

mod add_domain_to_workspace;
mod delete_domain_in_workspace;
mod get_domain_info_in_workspace;
mod get_verification_records_for_domain;
mod is_domain_valid;
mod list_domains_in_workspace;
mod verify_domain_in_workspace;

pub use self::{
	add_domain_to_workspace::*,
	delete_domain_in_workspace::*,
	get_domain_info_in_workspace::*,
	get_verification_records_for_domain::*,
	is_domain_valid::*,
	list_domains_in_workspace::*,
	verify_domain_in_workspace::*,
};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, allowed_client_type: ClientType) -> Router {
	Router::new()
		.mount_auth_endpoint(add_domain_to_workspace, state, allowed_client_type)
		.mount_auth_endpoint(list_domains_in_workspace, state, allowed_client_type)
		.mount_auth_endpoint(delete_domain_in_workspace, state, allowed_client_type)
		.mount_auth_endpoint(is_domain_valid, state, allowed_client_type)
		.mount_endpoint(is_domain_personal, state, allowed_client_type)
		.mount_auth_endpoint(add_dns_record, state, allowed_client_type)
		.mount_auth_endpoint(delete_dns_record, state, allowed_client_type)
		.mount_auth_endpoint(get_doamin_dns_record, state, allowed_client_type)
		.mount_auth_endpoint(get_domain_info_in_workspace, state, allowed_client_type)
		.mount_auth_endpoint(update_domain_dns_record, state, allowed_client_type)
		.mount_auth_endpoint(verify_domain_in_workspace, state, allowed_client_type)
		.mount_auth_endpoint(
			get_verification_records_for_domain,
			state,
			allowed_client_type,
		)
}

#[expect(unreachable_code, unused_variables)]
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
		state,
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

#[expect(unreachable_code, unused_variables)]
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
		user_data,
		state,
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

#[expect(unused_variables)]
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
		user_data,
		state,
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

#[expect(unreachable_code, unused_variables)]
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
		client_ip,
		user_data,
		state,
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

#[expect(unused_variables)]
async fn update_domain_dns_record(
	AuthenticatedAppRequest {
		request: ProcessedApiRequest {
			path,
			query: _,
			headers,
			body,
		},
		database,
		redis,
		client_ip,
		user_data,
		state,
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
