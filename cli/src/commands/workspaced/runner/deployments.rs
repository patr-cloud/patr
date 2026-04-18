use std::{collections::HashMap, path::PathBuf};

use clap::Args as ClapArgs;
use comfy_table::Table;
use common::prelude::{RunnerMode, RunnerSettings};
use docker::prelude::DockerSettings;
use inquire::Select;
use models::api::{
	user::*,
	workspace::{container_registry::*, deployment::*, runner::*},
};

use crate::prelude::*;

/// The arguments for the `runner deployments` command.
#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
	/// The type of runner configured on this host (ignored when `--runner` is
	/// set)
	#[arg(value_enum)]
	pub runner_type: RunnerType,
	/// Path to the runner config file (defaults to standard location)
	#[arg(short = 'c', long = "config")]
	pub config: Option<PathBuf>,
	/// List deployments for a specific runner by name or id. When set, the
	/// CLI's session token is used instead of the local runner config.
	#[arg(short = 'r', long = "runner")]
	pub runner: Option<String>,
}

/// List the deployments assigned to a runner.
pub(super) async fn execute(
	args: Args,
	global_args: GlobalArgs,
	state: AppState,
) -> Result<CommandOutput, AppError> {
	// Pick the auth + ids.
	// - When `--runner` is given, use the CLI's session token + workspace.
	// - Otherwise read the local runner config and use the runner's own api token.
	let (workspace_id, runner_id, token) = if let Some(runner_ref) = args.runner {
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

		let runners = make_request(
			ApiRequest::<ListRunnersForWorkspaceRequest>::builder()
				.path(ListRunnersForWorkspacePath { workspace_id })
				.headers(ListRunnersForWorkspaceRequestHeaders {
					authorization: token.clone(),
					user_agent: constants::USER_AGENT,
				})
				.build(),
		)
		.await?
		.body
		.runners;

		let id = Uuid::parse_str(&runner_ref).ok();
		let runner_id = runners
			.iter()
			.find(|r| r.name == runner_ref || id.filter(|id| r.id == *id).is_some())
			.map(|r| r.id)
			.ok_or_else(|| {
				AppError::RunnerError(format!("No runner found with name or ID: `{runner_ref}`"))
			})?;

		(workspace_id, runner_id, token)
	} else {
		match args.runner_type {
			RunnerType::Kubernetes => {
				todo!("Kubernetes runner is not yet supported")
			}
			RunnerType::Docker => {}
		}

		let config_path = args
			.config
			.unwrap_or_else(|| crate::utils::runner_config_path(args.runner_type));

		let config_str = std::fs::read_to_string(&config_path).map_err(|e| {
			AppError::RunnerError(format!(
				"Failed to read config file at {}: {e}",
				config_path.display()
			))
		})?;

		let config: RunnerSettings<DockerSettings> = serde_json::from_str(&config_str)
			.map_err(|e| AppError::RunnerError(format!("Failed to parse config: {e}")))?;

		match config.mode {
			RunnerMode::SelfHosted { .. } => {
				return Err(AppError::RunnerError(
					"This runner is configured in self-hosted mode; there is no managed runner id to filter deployments by. Pass `--runner <name|id>` to query a managed runner."
						.to_string(),
				));
			}
			RunnerMode::Managed {
				workspace_id,
				runner_id,
				api_token,
				..
			} => (workspace_id, runner_id, api_token),
		}
	};

	let deployments = make_request(
		ApiRequest::<ListDeploymentRequest>::builder()
			.path(ListDeploymentPath { workspace_id })
			.headers(ListDeploymentRequestHeaders {
				authorization: token.clone(),
				user_agent: constants::USER_AGENT,
			})
			.build(),
	)
	.await?
	.body
	.deployments
	.into_iter()
	.filter(|d| d.runner == runner_id)
	.collect::<Vec<_>>();

	let mut formatted_deployments = Vec::with_capacity(deployments.len());
	let mut runner_names = HashMap::<Uuid, String>::new();

	for deployment in &deployments {
		let runner_name = if let Some(name) = runner_names.get(&deployment.runner) {
			name.clone()
		} else {
			let name = make_request(
				ApiRequest::<GetRunnerInfoRequest>::builder()
					.path(GetRunnerInfoPath {
						workspace_id,
						runner_id: deployment.runner,
					})
					.headers(GetRunnerInfoRequestHeaders {
						authorization: token.clone(),
						user_agent: constants::USER_AGENT,
					})
					.build(),
			)
			.await?
			.body
			.runner
			.data
			.name;

			runner_names.insert(deployment.runner, name.clone());

			name
		};

		let image_name = match &deployment.registry {
			DeploymentRegistry::PatrRegistry {
				registry: PatrRegistry,
				repository_id,
			} => {
				let repo_name = make_request::<GetContainerRepositoryInfoRequest>(
					ApiRequest::builder()
						.path(GetContainerRepositoryInfoPath {
							workspace_id,
							repository_id: *repository_id,
						})
						.headers(GetContainerRepositoryInfoRequestHeaders {
							authorization: token.clone(),
							user_agent: constants::USER_AGENT,
						})
						.build(),
				)
				.await?
				.body
				.repository
				.name;

				format!("registry.patr.cloud/{}:{}", repo_name, deployment.image_tag)
			}
			DeploymentRegistry::ExternalRegistry {
				registry,
				image_name,
			} => {
				format!(
					"{}{}:{}",
					if registry != "docker.io" {
						format!("{registry}/")
					} else {
						Default::default()
					},
					image_name,
					deployment.data.image_tag
				)
			}
		};

		formatted_deployments.push([
			deployment.id.to_string(),
			deployment.name.clone(),
			image_name,
			runner_name,
			match deployment.status {
				DeploymentStatus::Running => "✅ Running",
				DeploymentStatus::Deploying => "🟡 Deploying",
				DeploymentStatus::Stopped => "🟧 Stopped",
				DeploymentStatus::Errored => "❌ Errored",
				DeploymentStatus::Unreachable => "❓ Unreachable",
			}
			.to_owned(),
			deployment
				.current_live_digest
				.clone()
				.unwrap_or_else(|| "-".to_string()),
		]);
	}

	CommandOutput::builder()
		.text(
			Table::new()
				.set_header([
					"ID",
					"Name",
					"Image",
					"Runner",
					"Status",
					"Current Image ID",
				])
				.add_rows(formatted_deployments)
				.to_string(),
		)
		.json(ListDeploymentResponse { deployments }.to_json_value())
		.build()
		.into_result()
}
