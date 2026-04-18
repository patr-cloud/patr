use std::{
	io::IsTerminal,
	process::{Command, Stdio},
};

use clap::Args as ClapArgs;
use inquire::{Confirm, Select};
use models::api::{user::*, workspace::container_registry::*};
use serde_json::Value;

use crate::prelude::*;

/// Args for `patr registry push`.
#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
	/// A local image reference (e.g. `my-app:latest` or a digest).
	pub source: String,
	/// Target Patr repository (name or id). Prompted if omitted.
	#[arg(short = 'r', long = "repo")]
	pub repo: Option<String>,
	/// Destination tag. Defaults to the tag in `source`, else prompts.
	#[arg(short = 't', long = "tag")]
	pub tag: Option<String>,
}

/// Tag a local image with the Patr registry URL and push it.
pub(super) async fn execute(
	args: Args,
	global_args: GlobalArgs,
	state: AppState,
) -> Result<CommandOutput, AppError> {
	let AppState::LoggedIn {
		token,
		current_workspace,
	} = state.clone()
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

		let workspace_name = global_args.workspace.clone().unwrap_or_else(|| {
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
				.unwrap_or_else(|| panic!("No repository found with name: `{selected}`"))
		});

	let tag = if let Some(tag) = args.tag {
		tag
	} else if let Some((_, rest)) = args.source.rsplit_once(':') &&
		!rest.is_empty() &&
		!rest.contains('/')
	{
		rest.to_string()
	} else if std::io::stdin().is_terminal() {
		let accept_latest = Confirm::new("No tag specified. Use 'latest'?")
			.with_default(true)
			.prompt()
			.expect_tty("Failed to read tag confirmation");
		if !accept_latest {
			return Err(AppError::RunnerError(
				"No tag specified. Re-run with `-t <tag>`.".to_string(),
			));
		}
		"latest".to_string()
	} else {
		return Err(AppError::RunnerError(
			"No tag specified and not running in a terminal. Pass `-t <tag>` explicitly."
				.to_string(),
		));
	};

	let target = format!("registry.patr.cloud/{}/{}:{}", workspace_id, repo.name, tag);

	let status = Command::new("docker")
		.args(["tag", &args.source, &target])
		.stdout(Stdio::inherit())
		.stderr(Stdio::inherit())
		.status()
		.map_err(|e| match e.kind() {
			std::io::ErrorKind::NotFound => {
				AppError::RunnerError("docker not found on PATH. Install Docker first.".to_string())
			}
			_ => AppError::RunnerError(format!("Failed to run `docker tag`: {e}")),
		})?;
	if !status.success() {
		return Err(AppError::RunnerError(format!(
			"`docker tag {} {}` failed (exit status {status}). Is the source image present locally?",
			args.source, target
		)));
	}

	let docker_logged_in = dirs::home_dir()
		.map(|home| home.join(".docker").join("config.json"))
		.and_then(|path| std::fs::read_to_string(path).ok())
		.and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
		.and_then(|v| {
			v.get("auths")
				.and_then(|a| a.get("registry.patr.cloud"))
				.cloned()
		})
		.is_some();

	if !docker_logged_in {
		let should_login = std::io::stdin().is_terminal() &&
			Confirm::new("Not logged in to registry.patr.cloud. Log in now?")
				.with_default(true)
				.prompt()
				.expect_tty("Failed to read login confirmation");

		if !should_login {
			return Err(AppError::RunnerError(
				"Not logged in to registry.patr.cloud. Run `patr registry login` first."
					.to_string(),
			));
		}

		super::login::execute(global_args.clone(), state.clone()).await?;
	}

	let status = Command::new("docker")
		.args(["push", &target])
		.stdout(Stdio::inherit())
		.stderr(Stdio::inherit())
		.status()
		.map_err(|e| AppError::RunnerError(format!("Failed to run `docker push`: {e}")))?;
	if !status.success() {
		return Err(AppError::RunnerError(format!(
			"`docker push {target}` failed (exit status {status}). \
			 If this is an auth error, try `patr registry login` first."
		)));
	}

	CommandOutput::builder()
		.text(format!("Pushed {target}"))
		.json(Value::String(target))
		.build()
		.into_result()
}
