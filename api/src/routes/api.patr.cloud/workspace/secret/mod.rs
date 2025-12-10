use axum::{Router, http::StatusCode};
use models::{ErrorType, api::workspace::secret::*};

use crate::prelude::*;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, allowed_client_type: ClientType) -> Router {
	Router::new()
		.mount_auth_endpoint(create_secret, state, allowed_client_type)
		.mount_auth_endpoint(delete_secret, state, allowed_client_type)
		.mount_auth_endpoint(list_secrets_for_workspace, state, allowed_client_type)
		.mount_auth_endpoint(update_secret, state, allowed_client_type)
		.with_state(state.clone())
}

async fn create_secret(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: CreateSecretPath { workspace_id },
				query: _,
				headers,
				body,
			},
		database,
		redis: _,
		client_ip: _,
		user_data,
		state,
	}: AuthenticatedAppRequest<'_, CreateSecretRequest>,
) -> Result<AppResponse<CreateSecretRequest>, ErrorType> {
	info!("Starting: Create secret");

	// LOGIC

	AppResponse::builder()
		.body(CreateSecretResponse { id: todo!() })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn delete_secret(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: DeleteSecretPath {
					workspace_id,
					secret_id,
				},
				query: _,
				headers: _,
				body,
			},
		database,
		redis: _,
		client_ip: _,
		user_data,
		state,
	}: AuthenticatedAppRequest<'_, DeleteSecretRequest>,
) -> Result<AppResponse<DeleteSecretRequest>, ErrorType> {
	info!("Starting: Delete secret");

	// LOGIC

	AppResponse::builder()
		.body(DeleteSecretResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn list_secrets_for_workspace(
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
	}: AuthenticatedAppRequest<'_, ListSecretsForWorkspaceRequest>,
) -> Result<AppResponse<ListSecretsForWorkspaceRequest>, ErrorType> {
	info!("Starting: List secrets for a workspace");

	// LOGIC

	AppResponse::builder()
		.body(ListSecretsForWorkspaceResponse { secrets: todo!() })
		.headers(ListSecretsForWorkspaceResponseHeaders {
			total_count: todo!(),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn update_secret(
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
	}: AuthenticatedAppRequest<'_, UpdateSecretRequest>,
) -> Result<AppResponse<UpdateSecretRequest>, ErrorType> {
	info!("Starting: Update secret");

	// LOGIC

	AppResponse::builder()
		.body(UpdateSecretResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
