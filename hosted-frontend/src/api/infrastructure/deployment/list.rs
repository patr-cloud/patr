use models::api::workspace::deployment::*;

use crate::prelude::*;

/// List Dpeloyments
#[server(ListDeploymentFn, endpoint = "/infrastructure/deployment/list")]
pub async fn list_deployments(
	access_token: Option<String>,
	workspace_id: Option<Uuid>,
) -> Result<ListDeploymentResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use constants::USER_AGENT_STRING;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = workspace_id
		.ok_or_else(|| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let api_response = make_api_call::<ListDeploymentRequest>(
		ApiRequest::builder()
			.path(ListDeploymentPath { workspace_id })
			.query(Paginated {
				data: (),
				page: 0,
				count: 10,
			})
			.headers(ListDeploymentRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static(USER_AGENT_STRING),
			})
			.body(ListDeploymentRequest)
			.build(),
	)
	.await;

	api_response
		.map(|res| res.body)
		.map_err(|err| ServerFnError::WrappedServerError(err))
}
