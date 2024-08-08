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
	(Option<String>, Option<String>),
	Result<ListDeploymentResponse, ServerFnError<ErrorType>>,
> {
	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);
	let (current_workspace_id, _) =
		use_cookie::<String, FromToStringCodec>(constants::LAST_USED_WORKSPACE_ID);

	// TODO: Use this with create_resource_with_initial_value
	let deployment_list = create_resource_with_initial_value(
		move || {
			logging::log!(
				"from get_deployment list: {:?}, {:?}",
				access_token.get(),
				current_workspace_id.get()
			);
			(access_token.get(), current_workspace_id.get())
		},
		move |(access_token, workspace_id)| async move {
			list_deployments(workspace_id, access_token).await
		},
		Some(Ok(ListDeploymentResponse {
			deployments: vec![],
		})),
	);

	deployment_list
}
