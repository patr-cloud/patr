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

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = Uuid::parse_str(workspace_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	make_api_call::<CreateDeploymentRequest>(
		ApiRequest::builder()
			.path(CreateDeploymentPath { workspace_id })
			.query(())
			.headers(CreateDeploymentRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static("todo"),
			})
			.body(deployment_info)
			.build(),
	)
	.await
	.map(|res| {
		leptos_axum::redirect("/deployment");
		res.body
	})
	.map_err(ServerFnError::WrappedServerError)
}
