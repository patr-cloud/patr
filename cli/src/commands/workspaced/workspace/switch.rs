use clap::Args as ClapArgs;
use inquire::Select;
use models::api::user::*;

use crate::prelude::*;

/// The arguments that can be passed to the switch workspace command.
#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
	/// Name of the workspace to switch to
	#[arg(
		short = 'w',
		alias = "name",
		value_name = "WORKSPACE_NAME_OR_ID",
		env = "PATR_WORKSPACE"
	)]
	pub workspace: Option<String>,
}

/// The command to switch between workspace contexts
pub(super) async fn execute(
	_: GlobalArgs,
	args: Args,
	state: AppState,
) -> Result<CommandOutput, AppError> {
	let AppState::LoggedIn {
		token,
		refresh_token,
		current_workspace: _,
	} = state
	else {
		return Err(AppError::NotLoggedIn);
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

	let workspace = args
		.workspace
		.and_then(|name| {
			let id = Uuid::parse_str(&name).ok();
			workspaces
				.iter()
				.find(|w| w.name == name || id.filter(|id| w.id == *id).is_some())
				.cloned()
		})
		.unwrap_or_else(|| {
			let name = Select::new(
				"Please select the workspace to switch to:",
				workspaces.iter().map(|workspace| &workspace.name).collect(),
			)
			.with_formatter(&|workspace| workspace.value.to_string())
			.prompt()
			.expect_tty("Failed to read workspace name / ID");

			workspaces
				.iter()
				.find(|&workspace| &workspace.name == name)
				.expect(&format!("No workspace found with name: `{}`", name))
				.clone()
		});

	AppState::LoggedIn {
		token,
		refresh_token,
		current_workspace: Some(workspace.id),
	}
	.save()?;

	CommandOutput {
		text: format!("Switched to workspace `{}`", workspace.name),
		json: ApiSuccessResponseBody::empty().to_json_value(),
	}
	.into_result()
}
