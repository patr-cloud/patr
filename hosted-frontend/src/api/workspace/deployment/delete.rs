use models::api::workspace::deployment::*;

use crate::prelude::*;

#[server(DeleteDeploymentFn, endpoint = "/infrastructure/deployment/delete")]
pub async fn delete_deployment(
	access_token: Option<String>,
	workspace_id: Option<Uuid>,
	deployment_id: Uuid,
) -> Result<DeleteDeploymentResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = workspace_id
		.ok_or_else(|| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	make_api_call::<DeleteDeploymentRequest>(
		ApiRequest::builder()
			.path(DeleteDeploymentPath {
				deployment_id,
				workspace_id,
			})
			.query(())
			.headers(DeleteDeploymentRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static("todo"),
			})
			.body(DeleteDeploymentRequest)
			.build(),
	)
	.await
	.map(|res| {
		leptos_axum::redirect("/deployment");
		res.body
	})
	.map_err(ServerFnError::WrappedServerError)
}
