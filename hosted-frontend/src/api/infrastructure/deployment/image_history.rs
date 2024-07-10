use models::api::workspace::deployment::deploy_history::*;

use crate::prelude::*;

#[server(
	DeploymentImageHistoryFn,
	endpoint = "/infrastructure/deployment/image-history"
)]
pub async fn get_deployment_image_history(
	access_token: Option<String>,
	deployment_id: Option<String>,
	workspace_id: Option<String>,
) -> Result<ListDeploymentDeployHistoryResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use constants::USER_AGENT_STRING;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = Uuid::parse_str(workspace_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let deployment_id = Uuid::parse_str(deployment_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let api_response = make_api_call::<ListDeploymentDeployHistoryRequest>(
		ApiRequest::builder()
			.path(ListDeploymentDeployHistoryPath {
				deployment_id,
				workspace_id,
			})
			.query(Paginated::default())
			.headers(ListDeploymentDeployHistoryRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static(USER_AGENT_STRING),
			})
			.body(ListDeploymentDeployHistoryRequest)
			.build(),
	)
	.await;

	api_response
		.map(|res| res.body)
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::InternalServerError))
}
