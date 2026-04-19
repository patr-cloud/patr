use clap::Args as ClapArgs;
use inquire::Select;
use models::api::{user::*, workspace::container_registry::*};
use serde_json::Value;

use crate::prelude::*;

/// The arguments for printing the registry URL
#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
	/// The name or ID of the repository
	#[arg(short = 'r', long = "repo")]
	pub repo: Option<String>,
}

/// Print the full registry image URL for a repository
pub(super) async fn execute(
	args: Args,
	global_args: GlobalArgs,
	state: AppState,
) -> Result<CommandOutput, AppError> {
	let AuthState::LoggedIn {
		token,
		current_workspace,
	} = state.auth
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

	// Resolve the repo to get its name
	let mut repositories = vec![];
	let mut start = 0;

	loop {
		let response = make_request(
			ApiRequest::<ListContainerRepositoriesRequest>::builder()
				.path(ListContainerRepositoriesPath { workspace_id })
				.headers(ListContainerRepositoriesRequestHeaders {
					authorization: token.clone(),
					user_agent: constants::USER_AGENT,
				})
				.query(ListResourceQuery {
					page: start / ListResourceQuery::DEFAULT_PAGE_SIZE,
					count: ListResourceQuery::DEFAULT_PAGE_SIZE,
					search: Default::default(),
					sort: Default::default(),
					additional_query: (),
				})
				.build(),
		)
		.await?;

		start += response.body.repositories.len();
		repositories.extend(response.body.repositories);

		if start >= response.headers.total_count.0 {
			break;
		}
	}

	let repo = args
		.repo
		.and_then(|name| {
			let id = Uuid::parse_str(&name).ok();
			repositories
				.iter()
				.find(|r| r.name == name || id.filter(|id| r.id == *id).is_some())
				.cloned()
		})
		.unwrap_or_else(|| {
			let names: Vec<String> = repositories.iter().map(|repo| repo.name.clone()).collect();
			let selected = Select::new("Please select a repository:", names)
				.prompt()
				.expect_tty("Failed to read repository");

			repositories
				.into_iter()
				.find(|repo| repo.name == selected)
				.unwrap_or_else(|| panic!("No repository found with name: `{}`", selected))
		});

	let url = format!("registry.patr.cloud/{}/{}", workspace_id, repo.name);

	CommandOutput::builder()
		.text(url.clone())
		.json(Value::String(url))
		.build()
		.into_result()
}
