use clap::{Args as ClapArgs};
use inquire::{Select, Text};
use models::api::{user::*, workspace::runner::*};

use crate::prelude::*;


#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
    #[arg(
        short = 'n',
        long = "name",
    )]
    pub name : Option<String>
}

pub async fn execute(
    args: Args,
	global_args: GlobalArgs,
	state: AppState,
)->Result<CommandOutput, AppError>{
    let AppState::LoggedIn {
		token,
		refresh_token: _,
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
     let name = args.name.unwrap_or_else(||{
        Text::new("Enter the name of runner :")
        .prompt()
        .expect_tty("Unable to read input")
    }); 

    let AddRunnerToWorkspaceResponse { id } = make_request(
        ApiRequest::<AddRunnerToWorkspaceRequest>::builder()
        .path(AddRunnerToWorkspacePath {workspace_id})
        .query(())
        .headers(AddRunnerToWorkspaceRequestHeaders{
            authorization:token,
            user_agent : UserAgent::from_static(constants::USER_AGENT_STRING)
        })
        .body(AddRunnerToWorkspaceRequest{name:name.clone()})
        .build(),
    ).await?.body;

    CommandOutput::builder()
    .text(format!("Runner `{}` created with ID `{}`",name,id.id))
    .json(ApiSuccessResponseBody::new(AddRunnerToWorkspaceResponse { id }).to_json_value())
    .build()
    .into_result()
}