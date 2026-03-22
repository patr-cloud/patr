use clap::Args as ClapArgs;
use inquire::{Confirm, Select};
use models::api::{user::*, workspace::container_registry::*};

use crate::prelude::*;

/// The arguments for deleting an image
#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
	/// The name or ID of the repository
	#[arg(short = 'r', long = "repo")]
	pub repo: Option<String>,
	/// The digest or tag of the image to delete
	#[arg(short = 'd', long = "digest")]
	pub digest: Option<String>,
}

/// Delete an image (manifest) from a container registry repository
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

	let digest_or_tag = if let Some(digest) = args.digest {
		digest
	} else {
		// List manifests and let user select
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

		let labels = manifests
			.iter()
			.map(|m| {
				if m.tags.is_empty() {
					format!("{} ({})", m.digest, m.platform)
				} else {
					format!("{} [{}] ({})", m.digest, m.tags.join(", "), m.platform)
				}
			})
			.collect::<Vec<_>>();

		let selected = Select::new("Please select an image to delete:", labels)
			.prompt()
			.expect_tty("Failed to read selection");

		manifests
			.iter()
			.find(|m| {
				let label = if m.tags.is_empty() {
					format!("{} ({})", m.digest, m.platform)
				} else {
					format!("{} [{}] ({})", m.digest, m.tags.join(", "), m.platform)
				};
				label == selected
			})
			.expect("Selected manifest not found")
			.digest
			.clone()
	};

	let confirmed = Confirm::new(&format!(
		"Are you sure you want to delete image `{}`?",
		digest_or_tag
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

	let response = make_request(
		ApiRequest::<DeleteContainerRepositoryManifestRequest>::builder()
			.path(DeleteContainerRepositoryManifestPath {
				workspace_id,
				repository_id,
				digest_or_tag: digest_or_tag.clone(),
			})
			.headers(DeleteContainerRepositoryManifestRequestHeaders {
				authorization: token,
				user_agent: constants::USER_AGENT,
			})
			.build(),
	)
	.await?
	.body;

	CommandOutput::builder()
		.text(format!("Deleted image `{}` successfully", digest_or_tag))
		.json(response.to_json_value())
		.build()
		.into_result()
}
