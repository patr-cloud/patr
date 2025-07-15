use clap::Args as ClapArgs;
use inquire::Text;
use models::{ApiSuccessResponseBody, api::workspace::*, prelude::*};

use crate::prelude::*;

/// The arguments that can be passed to the create workspace command.
#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
	/// Name of the workspace to be created
	#[arg(short = 'n', long = "name")]
	pub name: Option<String>,
}

/// The command to create a new workspace
pub(super) async fn execute(
	_: GlobalArgs,
	args: Args,
	state: AppState,
) -> Result<CommandOutput, AppError> {
	let AppState::LoggedIn {
		token,
		refresh_token: _,
		current_workspace: _,
	} = state
	else {
		return Err(AppError::NotLoggedIn);
	};

	// Check if the workspace name is provided
	let name = args.name.unwrap_or_else(|| {
		Text::new("Enter the name of the workspace:")
			.prompt()
			.expect_tty("Unable to read input")
	});

	let CreateWorkspaceResponse { id } = make_request(
		ApiRequest::<CreateWorkspaceRequest>::builder()
			.path(CreateWorkspacePath)
			.query(())
			.body(CreateWorkspaceRequest { name: name.clone() })
			.headers(CreateWorkspaceRequestHeaders {
				user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
				authorization: token,
			})
			.build(),
	)
	.await?
	.body;

	CommandOutput::builder()
		.text(format!("Workspace `{}` created with ID `{}`", name, id.id))
		.json(ApiSuccessResponseBody::new(CreateWorkspaceResponse { id }).to_json_value())
		.build()
		.into_result()
}
