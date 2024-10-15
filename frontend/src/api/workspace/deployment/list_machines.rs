use models::api::workspace::deployment::*;

use crate::prelude::*;

#[server(
	ListDeploymentMachinesFn,
	endpoint = "/infrastructure/deployment/machines/list"
)]
pub async fn list_all_machines(
	workspace_id: Uuid,
) -> Result<ListAllDeploymentMachineTypeResponse, ServerFnError<ErrorType>> {
	make_request::<ListAllDeploymentMachineTypeRequest>(
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
}
