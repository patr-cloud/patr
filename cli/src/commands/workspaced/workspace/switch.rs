use clap::Args;
use models::api::user::*;

use crate::prelude::*;

/// The arguments that can be passed to the switch workspace command.
#[derive(Debug, Clone, Args)]
pub struct SwitchArgs {
	/// Name of the workspace to switch to
	#[arg(short = 'n', long = "name")]
	pub name: String,
}

pub(super) async fn execute(
	_: GlobalArgs,
	args: SwitchArgs,
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
			"You are not logged in. Please log in to switch to a workspace.",
		));
	};

	let workspace = make_request(
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
	.workspaces
	.into_iter()
	.find(|workspace| workspace.name == args.name)
	.ok_or_else(|| {
		ApiErrorResponse::error_with_message(
			ErrorType::ResourceDoesNotExist,
			format!("Workspace `{}` not found.", args.name),
		)
	})?;

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
