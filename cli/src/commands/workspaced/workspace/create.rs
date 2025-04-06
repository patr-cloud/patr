use clap::Args;
use models::{ApiSuccessResponseBody, api::workspace::*, prelude::*};

use crate::prelude::*;

/// The arguments that can be passed to the create workspace command.
#[derive(Debug, Clone, Args)]
pub struct CreateArgs {
	/// Name of the workspace to be created
	#[arg(short = 'n', long = "name")]
	pub name: String,
}

pub(super) async fn execute(
	_: GlobalArgs,
	args: CreateArgs,
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
	let CreateWorkspaceResponse { id } = make_request(
		ApiRequest::<CreateWorkspaceRequest>::builder()
			.path(CreateWorkspacePath)
			.query(())
			.body(CreateWorkspaceRequest {
				name: args.name.clone(),
			})
			.headers(CreateWorkspaceRequestHeaders {
				user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
				authorization: token,
			})
			.build(),
	)
	.await?
	.body;

	CommandOutput {
		text: format!("Workspace `{}` created with ID `{}`", args.name, id.id),
		json: ApiSuccessResponseBody::new(CreateWorkspaceResponse { id }).to_json_value(),
	}
	.into_result()
}
