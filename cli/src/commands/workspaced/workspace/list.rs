use comfy_table::Table;
use models::api::user::*;

use crate::prelude::*;

pub(super) async fn execute(
	global_args: GlobalArgs,
	(): (),
	state: AppState,
) -> Result<CommandOutput, ApiErrorResponse> {
	let AppState::LoggedIn {
		token,
		refresh_token,
		current_workspace: _,
	} = state
	else {
		return Err(ApiErrorResponse::error_with_message(
			ErrorType::Unauthorized,
			"You are not logged in. Please log in to list workspaces.",
		));
	};

	let workspaces = make_request(
		ApiRequest::<ListUserWorkspacesRequest>::builder()
			.path(ListUserWorkspacesPath)
			.headers(ListUserWorkspacesRequestHeaders {
				authorization: token.clone(),
				user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
			})
			.query(())
			.body(ListUserWorkspacesRequest)
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
				.query(())
				.body(GetUserDetailsRequest)
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

	CommandOutput {
		text: Table::new()
			.set_header(["ID", "Name", "Super Admin"])
			.add_rows(formatted_workspaces)
			.to_string(),
		json: ListUserWorkspacesResponse { workspaces }.to_json_value(),
	}
	.into_result()
}
