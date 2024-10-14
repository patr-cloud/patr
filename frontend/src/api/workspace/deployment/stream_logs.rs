use crate::prelude::*;

/// Server function for streaming deployment logs.
#[server(
	StreamDeploymentLogsFn,
	endpoint = "/infrastructure/deployment/stream_logs"
)]
pub async fn stream_deployment_logs(
	access_token: Option<String>,
	workspace_id: Option<Uuid>,
	_deployment_id: Uuid,
) -> Result<(), ServerFnError<ErrorType>> {
	use std::str::FromStr;

	let access_token = access_token
		.ok_or_else(|| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;
	let _access_token = BearerToken::from_str(access_token.as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let _workspace_id = workspace_id
		.ok_or_else(|| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	// let x = make_api_call::<StreamDeploymentLogsRequest>(
	// 	ApiRequest::builder()
	// 		.path(StreamDeploymentLogsPath {
	// 			deployment_id,
	// 			workspace_id,
	// 		})
	// 		.query(StreamDeploymentLogsQuery { start_time: None })
	// 		.headers(StreamDeploymentLogsRequestHeaders {
	// 			authorization: access_token,
	// 			user_agent: UserAgent::from_static("todo"),
	// 		})
	// 		.body(Default::default())
	// 		.build(),
	// )
	// .await
	// .map(|res| res.body)
	// .map_err(ServerFnError::WrappedServerError);
	Err(ServerFnError::WrappedServerError(ErrorType::server_error(
		"Not Implemented",
	)))
}
