use clap::Args as ClapArgs;
use comfy_table::Table;
use inquire::Select;
use models::api::{user::*, workspace::container_registry::*};

use crate::prelude::*;

/// The arguments for listing images
#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
	/// The name or ID of the repository
	#[arg(short = 'r', long = "repo")]
	pub repo: Option<String>,
}

/// List images (manifests) in a container registry repository
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

	let repository_id = args
		.repo
		.and_then(|name| {
			let id = Uuid::parse_str(&name).ok();
			repositories
				.iter()
				.find(|r| r.name == name || id.filter(|id| r.id == *id).is_some())
				.map(|repo| repo.id)
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
				.id
		});

	let mut manifests = vec![];
	let mut start = 0;

	loop {
		let response = make_request(
			ApiRequest::<ListContainerRepositoryManifestsRequest>::builder()
				.path(ListContainerRepositoryManifestsPath {
					workspace_id,
					repository_id,
				})
				.headers(ListContainerRepositoryManifestsRequestHeaders {
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

		start += response.body.manifests.len();
		manifests.extend(response.body.manifests);

		if start >= response.headers.total_count.0 {
			break;
		}
	}

	let formatted_rows: Vec<[String; 5]> = manifests
		.iter()
		.map(|manifest| {
			[
				manifest.digest.clone(),
				format_size(manifest.size),
				manifest.platform.clone(),
				manifest.tags.join(", "),
				manifest.created.to_string(),
			]
		})
		.collect();

	CommandOutput::builder()
		.text(
			Table::new()
				.set_header(["Digest", "Size", "Platform", "Tags", "Created"])
				.add_rows(formatted_rows)
				.to_string(),
		)
		.json(ListContainerRepositoryManifestsResponse { manifests }.to_json_value())
		.build()
		.into_result()
}

fn format_size(bytes: u64) -> String {
	const KB: u64 = 1024;
	const MB: u64 = KB * 1024;
	const GB: u64 = MB * 1024;

	if bytes >= GB {
		format!("{:.1} GB", bytes as f64 / GB as f64)
	} else if bytes >= MB {
		format!("{:.1} MB", bytes as f64 / MB as f64)
	} else if bytes >= KB {
		format!("{:.1} KB", bytes as f64 / KB as f64)
	} else {
		format!("{} B", bytes)
	}
}
