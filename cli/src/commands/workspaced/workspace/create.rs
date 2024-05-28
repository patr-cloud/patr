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
	global_args: GlobalArgs,
	args: CreateArgs,
	state: AppState,
) -> Result<CommandOutput, ApiErrorResponse> {
	let AppState::LoggedIn {
		token,
		refresh_token,
	} = state
	else {
		return Err(ApiErrorResponse::error_with_message(
			ErrorType::Unauthorized,
			"You are not logged in. Please log in to create a workspace.",
		));
	};
	let CreateWorkspaceResponse { workspace_id } = make_request(
		ApiRequest::<CreateWorkspaceRequest>::builder()
			.path(CreateWorkspacePath)
			.query(())
			.body(CreateWorkspaceRequest {
				workspace_name: args.name.clone(),
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
		text: format!(
			"Workspace `{}` created with ID `{}`",
			args.name, workspace_id
		),
		json: ApiSuccessResponseBody::new(CreateWorkspaceResponse { workspace_id }).to_json_value(),
	}
	.into_result()
}
