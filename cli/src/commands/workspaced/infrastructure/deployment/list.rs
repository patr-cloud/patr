use std::collections::HashMap;

use comfy_table::Table;
use inquire::Select;
use models::api::{
	user::*,
	workspace::{container_registry::*, deployment::*, runner::*},
};

use crate::prelude::*;

/// The command to list all workspaces that the user is a part of
pub(super) async fn execute(
	global_args: GlobalArgs,
	state: AppState,
) -> Result<CommandOutput, AppError> {
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
				.headers(ListUserWorkspacesRequestHeaders {
					authorization: token.clone(),
					user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
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

	let deployments = make_request(
		ApiRequest::<ListDeploymentRequest>::builder()
			.path(ListDeploymentPath { workspace_id })
			.headers(ListDeploymentRequestHeaders {
				authorization: token.clone(),
				user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
			})
			.build(),
	)
	.await?
	.body
	.deployments;

	let mut formatted_deployments = Vec::with_capacity(deployments.len());
	let mut runners = HashMap::<Uuid, String>::new();

	for deployment in &deployments {
		let runner_name = if let Some(name) = runners.get(&deployment.runner) {
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
						user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
					})
					.build(),
			)
			.await?
			.body
			.runner
			.data
			.name;

			runners.insert(deployment.runner, name.clone());

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
							user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
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
