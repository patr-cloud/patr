use models::api::workspace::deployment::ListDeploymentResponse;

use crate::prelude::*;

/// Get the list of deployments as a resource
pub fn get_deployments() -> Resource<Result<ListDeploymentResponse, ServerFnError<ErrorType>>> {
	let (state, _) = AuthState::load();
	let access_token = state.get().get_access_token();
	let workspace_id = state.get().get_last_used_workspace_id();

	Resource::new(
		move || (access_token.clone(), workspace_id),
		move |(access_token, workspace_id)| async move {
			list_deployments(access_token, workspace_id.unwrap(), None, None)
				.await
				.map(|(_, body)| body)
		},
		// Some(Ok(ListDeploymentResponse {
		// 	deployments: vec![],
		// })),
	)
}
