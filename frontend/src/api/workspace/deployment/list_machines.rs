use models::api::workspace::deployment::*;

use crate::prelude::*;

#[server(
	ListDeploymentMachinesFn,
	endpoint = "/infrastructure/deployment/machines/list"
)]
pub async fn list_all_machines(
	workspace_id: Option<Uuid>,
) -> Result<ListAllDeploymentMachineTypeResponse, ServerFnError<ErrorType>> {
	let workspace_id = workspace_id
		.ok_or_else(|| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	make_api_call::<ListAllDeploymentMachineTypeRequest>(
		ApiRequest::builder()
			.path(ListAllDeploymentMachineTypePath { workspace_id })
			.query(())
			.headers(ListAllDeploymentMachineTypeRequestHeaders {
				user_agent: UserAgent::from_static("todo"),
			})
			.body(ListAllDeploymentMachineTypeRequest)
			.build(),
	)
	.await
	.map(|res| res.body)
	.map_err(ServerFnError::WrappedServerError)
}
