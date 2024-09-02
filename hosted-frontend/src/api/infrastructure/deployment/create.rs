use leptos::server_fn::codec::Json;
use models::api::workspace::deployment::*;

use crate::prelude::*;

/// Server function to create a deployment
#[server(
	CreateDeploymentFn,
	input = Json,
	endpoint = "/infrastructure/deployment/create"
)]
pub async fn create_deployment(
	access_token: Option<String>,
	workspace_id: Option<String>,
	deployment_info: CreateDeploymentRequest,
) -> Result<CreateDeploymentResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use constants::USER_AGENT_STRING;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = Uuid::parse_str(workspace_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let api_response = make_api_call::<CreateDeploymentRequest>(
		ApiRequest::builder()
			.path(CreateDeploymentPath { workspace_id })
			.query(())
			.headers(CreateDeploymentRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static(USER_AGENT_STRING),
			})
			.body(deployment_info)
			.build(),
	)
	.await;

	if api_response.is_ok() {
		leptos_axum::redirect("/deployment");
	}

	api_response
		.map(|res| res.body)
		.map_err(|err| ServerFnError::WrappedServerError(err))
}
