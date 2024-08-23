use codee::string::FromToStringCodec;
use models::api::workspace::deployment::ListDeploymentResponse;

use crate::prelude::*;

pub fn get_workspaces() {
	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);

	let workspace_list = create_resource(
		move || access_token.get(),
		move |value| async move { list_user_workspace(value).await },
	);

	let x = workspace_list.get().unwrap().unwrap();
}

/// Get the list of deployments as a resource
pub fn get_deployments() -> Resource<
	(Option<String>, Option<Uuid>),
	Result<ListDeploymentResponse, ServerFnError<ErrorType>>,
> {
	let (state, _) = AuthState::load();
	let access_token = state.get().get_access_token();
	let workspace_id = state.get().get_last_used_workspace_id();

	// TODO: Use this with create_resource_with_initial_value

	create_resource_with_initial_value(
		move || (access_token.clone(), workspace_id),
		move |(access_token, workspace_id)| async move {
			list_deployments(access_token, workspace_id).await
		},
		Some(Ok(ListDeploymentResponse {
			deployments: vec![],
		})),
	)
}
