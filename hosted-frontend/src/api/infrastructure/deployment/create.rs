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
	// The deployment info is of type CreateDeploymentRequest Converted to a serde_json::Value
	deployment_info: CreateDeploymentRequest,
) -> Result<CreateDeploymentResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use constants::USER_AGENT_STRING;

	logging::log!("before serialize: {:#?}", deployment_info);
	// let mut deployment_info = deployment_info;
	// *deployment_info
	// 	.as_object_mut()
	// 	.unwrap()
	// 	.get_mut("deployOnCreate")
	// 	.unwrap() = serde_json::Value::Bool(false);
	// *deployment_info
	// 	.as_object_mut()
	// 	.unwrap()
	// 	.get_mut("minHorizontalScale")
	// 	.unwrap() = serde_json::Value::Number(1.into());
	// *deployment_info
	// 	.as_object_mut()
	// 	.unwrap()
	// 	.get_mut("maxHorizontalScale")
	// 	.unwrap() = serde_json::Value::Number(10.into());
	// *deployment_info
	// 	.as_object_mut()
	// 	.unwrap()
	// 	.get_mut("deployOnPush")
	// 	.unwrap() = serde_json::Value::Bool(false);
	// let deployment_info = serde_json::from_value(deployment_info).unwrap();
	// logging::log!("after serialize: {:#?}", deployment_info);

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
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::InternalServerError))
}
