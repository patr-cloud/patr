use comfy_table::Table;
use inquire::Select;
use models::api::{user::*, workspace::runner::*};

use crate::prelude::*;

/// The command to list all runners in a workspace
pub(super) async fn execute(
	global_args: GlobalArgs,
	state: AppState,
) -> Result<CommandOutput, AppError> {
	let AppState::LoggedIn {
		token,
		current_workspace,
	} = state
	else {
		return Err(AppError::NotLoggedIn);
	};

	let workspace_id = if let Some(workspace_id) = current_workspace {
		workspace_id
	} else {
		let workspaces = make_request(
			ApiRequest::<ListUserWorkspacesRequest>::builder()
				.headers(ListUserWorkspacesRequestHeaders {
					authorization: token.clone(),
					user_agent: constants::USER_AGENT,
				})
				.build(),
		)
		.await?
		.body
		.workspaces;

		let workspace_name = global_args.workspace.unwrap_or_else(|| {
			Select::new(
				"Please select a workspace to use",
				workspaces
					.iter()
					.map(|workspace| workspace.name.clone())
					.collect(),
			)
			.prompt()
			.expect_tty("Failed to read workspace ID")
		});

		workspaces
			.into_iter()
			.find(|workspace| {
				workspace.id.to_string() == workspace_name || workspace.name == workspace_name
			})
			.unwrap_or_else(|| panic!("No workspace found with ID or name: `{workspace_name}`"))
			.id
	};

	let runners = make_request(
		ApiRequest::<ListRunnersForWorkspaceRequest>::builder()
			.path(ListRunnersForWorkspacePath { workspace_id })
			.headers(ListRunnersForWorkspaceRequestHeaders {
				authorization: token.clone(),
				user_agent: constants::USER_AGENT,
			})
			.build(),
	)
	.await?
	.body
	.runners;

	let mut formatted_runners = Vec::with_capacity(runners.len());

	for runner in &runners {
		let connected = make_request(
			ApiRequest::<GetRunnerInfoRequest>::builder()
				.path(GetRunnerInfoPath {
					workspace_id,
					runner_id: runner.id,
				})
				.headers(GetRunnerInfoRequestHeaders {
					authorization: token.clone(),
					user_agent: constants::USER_AGENT,
				})
				.build(),
		)
		.await?
		.body
		.runner
		.data
		.connected;
		formatted_runners.push([
			runner.id.to_string(),
			runner.name.clone(),
			match connected {
				true => "✅ Connected",
				false => "❌ Disconnected",
			}
			.to_owned(),
		])
	}

	CommandOutput::builder()
		.text(
			Table::new()
				.set_header(["ID", "Name", "Connected"])
				.add_rows(formatted_runners)
				.to_string(),
		)
		.json(ListRunnersForWorkspaceResponse { runners }.to_json_value())
		.build()
		.into_result()
}
