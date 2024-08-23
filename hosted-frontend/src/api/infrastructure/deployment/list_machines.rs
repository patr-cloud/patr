use models::api::workspace::deployment::*;

use crate::prelude::*;

#[server(
	ListDeploymentMachinesFn,
	endpoint = "/infrastructure/deployment/machines/list"
)]
pub async fn list_all_machines(
	workspace_id: Option<String>,
) -> Result<ListAllDeploymentMachineTypeResponse, ServerFnError<ErrorType>> {
	use constants::USER_AGENT_STRING;

	let workspace_id = Uuid::parse_str(workspace_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let api_response = make_api_call::<ListAllDeploymentMachineTypeRequest>(
		ApiRequest::builder()
			.path(ListAllDeploymentMachineTypePath { workspace_id })
			.query(())
			.headers(ListAllDeploymentMachineTypeRequestHeaders {
				user_agent: UserAgent::from_static(USER_AGENT_STRING),
			})
			.body(ListAllDeploymentMachineTypeRequest)
			.build(),
	)
	.await;

	api_response
		.map(|res| res.body)
		.map_err(|err| ServerFnError::WrappedServerError(err))
}
