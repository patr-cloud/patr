use clap::Args;
use models::{api::workspace::*, prelude::*, ApiErrorResponse, ApiSuccessResponseBody};

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
) -> Result<CommandOutput, ApiErrorResponse> {
	let AppState::LoggedIn {
		token,
		refresh_token,
		current_workspace: _,
	} = state
	else {
		return Err(ApiErrorResponse::error_with_message(
			ErrorType::Unauthorized,
			"You are not logged in. Please log in to create a workspace.",
		));
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
