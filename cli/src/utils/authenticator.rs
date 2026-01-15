use std::str::FromStr;

use inquire::Text;
use models::api::workspace::*;

use crate::prelude::*;

/// The arguments to the CLI after getting the workspace details and
/// authenticating the user
pub struct WorkspacedArgs {
	/// The workspace to use for the command
	pub workspace: WithId<Workspace>,
	/// The token used to authenticate with the API
	pub token: BearerToken,
}

impl WorkspacedArgs {
	/// Generates the workspace arguments from the global arguments and the
	/// state
	pub async fn generate(global_args: &GlobalArgs, state: &AppState) -> Result<Self, AppError> {
		let token = if let Some(token) = &global_args.token {
			BearerToken::from_str(token).map_err(|err| AppError::ParseError(err.to_string()))?
		} else {
			let AppState::LoggedIn {
				token,
				current_workspace: _,
			} = state
			else {
				return Err(AppError::NotLoggedIn);
			};

			token.clone()
		};

		let workspace_id = if let Some(workspace_id) = &global_args.workspace {
			workspace_id
				.parse::<Uuid>()
				.map_err(|err| AppError::ParseError(err.to_string()))?
		} else {
			let AppState::LoggedIn {
				token: _,
				current_workspace,
			} = state
			else {
				return Err(AppError::NoWorkspace);
			};

			current_workspace.unwrap_or_else(|| {
				Text::new("Please enter the workspace ID you want to use:")
					.prompt()
					.expect_tty("Failed to read workspace ID")
					.parse::<Uuid>()
					.expect("Failed to parse workspace ID")
			})
		};

		let workspace = make_request(
			ApiRequest::<GetWorkspaceInfoRequest>::builder()
				.path(GetWorkspaceInfoPath { workspace_id })
				.headers(GetWorkspaceInfoRequestHeaders {
					authorization: token.clone(),
					user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
				})
				.build(),
		)
		.await?
		.body
		.workspace;

		Ok(Self { workspace, token })
	}
}
