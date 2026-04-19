use clap::Args as ClapArgs;
use inquire::{Confirm, Select};
use models::api::{user::*, workspace::container_registry::*};

use crate::prelude::*;

/// The arguments for deleting a container registry repository
#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
	/// The name or ID of the repository to delete
	#[arg(short = 'n', long = "name")]
	pub name: Option<String>,
}

/// Delete a container registry repository
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

	let repository = args
		.name
		.and_then(|name| {
			let id = Uuid::parse_str(&name).ok();
			repositories
				.iter()
				.find(|r| r.name == name || id.filter(|id| r.id == *id).is_some())
		})
		.unwrap_or_else(|| {
			let name = Select::new(
				"Please select a repository:",
				repositories.iter().map(|repo| &repo.name).collect(),
			)
			.with_formatter(&|repo| repo.value.to_string())
			.prompt()
			.expect_tty("Failed to read repository");

			repositories
				.iter()
				.find(|repo| &repo.name == name)
				.unwrap_or_else(|| panic!("No repository found with name: `{}`", name))
		});

	let confirmed = Confirm::new(&format!(
		"Are you sure you want to delete registry.patr.cloud/{}/{}?",
		workspace_id, repository.name
	))
	.with_default(false)
	.prompt()
	.expect_tty("Failed to read confirmation");

	if !confirmed {
		return CommandOutput::builder()
			.text("Aborted.".to_string())
			.json(serde_json::Value::Null)
			.build()
			.into_result();
	}

	let repository_id = repository.id;

	let response = make_request(
		ApiRequest::<DeleteContainerRepositoryRequest>::builder()
			.path(DeleteContainerRepositoryPath {
				workspace_id,
				repository_id,
			})
			.headers(DeleteContainerRepositoryRequestHeaders {
				authorization: token,
				user_agent: constants::USER_AGENT,
			})
			.build(),
	)
	.await?
	.body;

	CommandOutput::builder()
		.text(format!(
			"Deleted repository `{}` successfully",
			repository_id
		))
		.json(response.to_json_value())
		.build()
		.into_result()
}
