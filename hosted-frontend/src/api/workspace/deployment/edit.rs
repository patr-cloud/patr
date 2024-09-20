use models::api::workspace::deployment::*;

use crate::prelude::*;

/// The Server Function for updating a deployment
#[server(UpdateDeploymentFn, endpoint = "/infrastructure/deployment/update")]
pub async fn update_deployment(
	access_token: Option<String>,
	workspace_id: Option<Uuid>,
	deployment_id: Option<Uuid>,
	deployment_info: UpdateDeploymentRequest,
) -> Result<UpdateDeploymentResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = workspace_id
		.ok_or_else(|| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let deployment_id = deployment_id
		.ok_or_else(|| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	make_api_call::<UpdateDeploymentRequest>(
		ApiRequest::builder()
			.path(UpdateDeploymentPath {
				workspace_id,
				deployment_id,
			})
			.query(())
			.headers(UpdateDeploymentRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static("todo"),
			})
			.body(deployment_info)
			.build(),
	)
	.await
	.map(|res| res.body)
	.map_err(ServerFnError::WrappedServerError)
}
