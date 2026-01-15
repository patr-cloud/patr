use comfy_table::Table;
use models::api::user::*;

use crate::prelude::*;

/// The command to list all workspaces that the user is a part of
pub(super) async fn execute(
	_global_args: GlobalArgs,
	state: AppState,
) -> Result<CommandOutput, AppError> {
	let AppState::LoggedIn {
		token,
		current_workspace: _,
	} = state
	else {
		return Err(AppError::NotLoggedIn);
	};

	let workspaces = make_request(
		ApiRequest::<ListUserWorkspacesRequest>::builder()
			.headers(ListUserWorkspacesRequestHeaders {
				authorization: token.clone(),
				user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
			})
			.build(),
	)
	.await?
	.body
	.workspaces;

	let mut formatted_workspaces = Vec::with_capacity(workspaces.len());

	for workspace in &workspaces {
		let super_admin = make_request(
			ApiRequest::<GetUserDetailsRequest>::builder()
				.path(GetUserDetailsPath {
					user_id: workspace.super_admin_id,
				})
				.headers(GetUserDetailsRequestHeaders {
					authorization: token.clone(),
					user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
				})
				.build(),
		)
		.await?
		.body
		.basic_user_info;

		formatted_workspaces.push([
			workspace.id.to_string(),
			workspace.name.to_owned(),
			format!("{} {}", super_admin.first_name, super_admin.last_name),
		]);
	}

	CommandOutput::builder()
		.text(
			Table::new()
				.set_header(["ID", "Name", "Super Admin"])
				.add_rows(formatted_workspaces)
				.to_string(),
		)
		.json(ListUserWorkspacesResponse { workspaces }.to_json_value())
		.build()
		.into_result()
}
