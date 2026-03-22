use clap::Args as ClapArgs;
use inquire::{Select, Text};
use models::api::{user::*, workspace::container_registry::*};

use crate::prelude::*;

/// The arguments for creating a container registry repository
#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
	/// The name of the repository to create
	#[arg(short = 'n', long = "name")]
	pub name: Option<String>,
}

/// Create a new container registry repository
pub(super) async fn execute(
	args: Args,
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

	let name = args.name.unwrap_or_else(|| {
		Text::new("Enter the name of the repository:")
			.with_help_message(&format!(
				"Your repository will be created as `registry.patr.cloud/{}/{{name}}`",
				workspace_id
			))
			.prompt()
			.expect_tty("Unable to read input")
	});

	let CreateContainerRepositoryResponse { id } = make_request(
		ApiRequest::<CreateContainerRepositoryRequest>::builder()
			.path(CreateContainerRepositoryPath { workspace_id })
			.headers(CreateContainerRepositoryRequestHeaders {
				authorization: token,
				user_agent: constants::USER_AGENT,
			})
			.body(CreateContainerRepositoryRequest { name: name.clone() })
			.build(),
	)
	.await?
	.body;

	CommandOutput::builder()
		.text(format!("Repository `{}` created with ID `{}`", name, id.id))
		.json(ApiSuccessResponseBody::new(CreateContainerRepositoryResponse { id }).to_json_value())
		.build()
		.into_result()
}
