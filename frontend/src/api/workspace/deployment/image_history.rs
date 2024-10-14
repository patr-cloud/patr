use models::api::workspace::deployment::deploy_history::*;

use crate::prelude::*;

#[server(
	DeploymentImageHistoryFn,
	endpoint = "/infrastructure/deployment/image-history"
)]
pub async fn get_deployment_image_history(
	access_token: Option<String>,
	deployment_id: Uuid,
	workspace_id: Uuid,
) -> Result<ListDeploymentDeployHistoryResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	make_api_call::<ListDeploymentDeployHistoryRequest>(
		ApiRequest::builder()
			.path(ListDeploymentDeployHistoryPath {
				deployment_id,
				workspace_id,
			})
			.query(Paginated::default())
			.headers(ListDeploymentDeployHistoryRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static("todo"),
			})
			.body(ListDeploymentDeployHistoryRequest)
			.build(),
	)
	.await
	.map(|res| res.body)
	.map_err(ServerFnError::WrappedServerError)
}
