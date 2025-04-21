use std::io::IsTerminal;

use clap::{ArgAction, Args};
use inquire::{Confirm, Select, Text};
use models::api::{
	user::*,
	workspace::{container_registry::*, deployment::*, runner::*},
};

use crate::{prelude::*, utils::StringExt};

#[derive(Debug, Clone, Args)]
pub struct CreateArgs {
	/// The name of the deployment
	#[arg(
		short = 'n',
		long = "name",
		value_name = "NAME",
		env = "PATR_DEPLOYMENT_NAME"
	)]
	pub name: Option<String>,
	/// The registry to use for the deployment
	#[arg(
		short = 'r',
		long = "registry",
		value_name = "REGISTRY",
		env = "PATR_DEPLOYMENT_REGISTRY"
	)]
	pub registry: Option<String>,
	/// The image to use for the deployment
	#[arg(
		short = 'i',
		long = "image",
		value_name = "IMAGE",
		env = "PATR_DEPLOYMENT_IMAGE"
	)]
	pub image: Option<String>,
	/// The tag of the image to use for the deployment
	#[arg(
		short = 't',
		long = "tag",
		value_name = "TAG",
		env = "PATR_DEPLOYMENT_TAG"
	)]
	pub tag: Option<String>,
	/// The machine type to use for the deployment
	#[arg(
		short = 'm',
		long = "machine-type",
		value_name = "MACHINE-TYPE",
		env = "PATR_DEPLOYMENT_MACHINE_TYPE"
	)]
	pub machine_type: Option<String>,
	/// The runner to use for the deployment
	#[arg(
		long = "runner",
		value_name = "RUNNER-NAME-OR-ID",
		env = "PATR_DEPLOYMENT_RUNNER"
	)]
	pub runner: Option<String>,
	/// Whether to deploy on push
	#[arg(
		long = "deploy-on-push",
		env = "PATR_DEPLOYMENT_DEPLOY_ON_PUSH",
		action = ArgAction::SetTrue,
	)]
	pub deploy_on_push: Option<bool>,
}

pub async fn execute(
	args: CreateArgs,
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
		let workspace_name = global_args.workspace.unwrap_or_else(|| {
			Text::new("Please enter the workspace you want to use:")
				.prompt()
				.expect_tty("Failed to read workspace ID")
		});

		make_request(
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
		.workspaces
		.into_iter()
		.find(|workspace| {
			workspace.id.to_string() == workspace_name || workspace.name == workspace_name
		})
		.unwrap_or_else(|| panic!("No workspace found with ID or name: `{}`", workspace_name))
		.id
	};

	let name = args.name.unwrap_or_else(|| {
		Text::new("Please enter the deployment name:")
			.prompt()
			.expect_tty("Failed to read deployment name")
			.to_string()
	});

	let registry = args.registry.unwrap_or_else(|| {
		Text::new("Please enter the registry name:")
			.with_autocomplete(|text: &str| {
				let results = vec![
					"registry.patr.cloud".to_string(),
					"docker.io".to_string(),
					"ghcr.io".to_string(),
				]
				.into_iter()
				.filter(|name| name.to_lowercase().contains(&text.to_lowercase()))
				.collect();

				Ok(results)
			})
			.prompt()
			.expect_tty("Failed to read registry name")
			.to_string()
	});

	let registry = if registry == "registry.patr.cloud" {
		let mut repositories = vec![];
		let mut start = 0;

		loop {
			let response = make_request(
				ApiRequest::<ListContainerRepositoriesRequest>::builder()
					.path(ListContainerRepositoriesPath { workspace_id })
					.headers(ListContainerRepositoriesRequestHeaders {
						authorization: token.clone(),
						user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
					})
					.query(Paginated {
						page: start / Paginated::DEFAULT_PAGE_SIZE,
						count: Paginated::DEFAULT_PAGE_SIZE,
						data: (),
					})
					.body(ListContainerRepositoriesRequest)
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
			.image
			.and_then(|image| {
				let id = Uuid::parse_str(&image).ok();
				repositories
					.iter()
					.find(|repo| repo.name == image || id.filter(|id| repo.id == *id).is_some())
					.map(|repo| repo.id)
			})
			.unwrap_or_else(|| {
				let name = Select::new(
					"Please select the repository to use:",
					repositories.iter().map(|repo| &repo.name).collect(),
				)
				.with_formatter(&|repo_id| {
					format!("registry.patr.cloud/{workspace_id}/{}", repo_id.value)
				})
				.prompt()
				.expect_tty("Failed to read repository ID");

				repositories
					.iter()
					.find(|&repo| &repo.name == name)
					.expect(&format!("No repository found with name: `{}`", name))
					.id
			});

		DeploymentRegistry::PatrRegistry {
			registry: PatrRegistry,
			repository_id,
		}
	} else {
		let image_name = args.image.unwrap_or_else(|| {
			Text::new("Please enter the image name:")
				.prompt()
				.expect_tty("Failed to read image name")
		});

		DeploymentRegistry::ExternalRegistry {
			registry,
			image_name,
		}
	};

	let image_tag = args
		.tag
		.unwrap_or_else(|| {
			Text::new("Please enter the image tag:")
				.with_help_message("The tag of the image to use (eg: `latest`, `stable`, etc)")
				.prompt()
				.expect_tty("Failed to read image tag")
		})
		.some_if_not_empty()
		.unwrap_or_else(|| "latest".to_string());

	let runners = make_request(
		ApiRequest::<ListRunnersForWorkspaceRequest>::builder()
			.path(ListRunnersForWorkspacePath { workspace_id })
			.query(Paginated::default())
			.headers(ListRunnersForWorkspaceRequestHeaders {
				user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
				authorization: token.clone(),
			})
			.body(ListRunnersForWorkspaceRequest)
			.build(),
	)
	.await?
	.body
	.runners;

	let runner = args
		.runner
		.map(|runner| {
			runners
				.iter()
				.find(|r| r.id.to_string() == runner || r.name == runner)
				.expect(&format!("No runner found with ID or name: `{}`", runner))
				.id
		})
		.unwrap_or_else(|| {
			let name = Select::new(
				"Select the runner to use: ",
				runners.iter().map(|runner| &runner.name).collect(),
			)
			.with_help_message("The runner to use for the deployment")
			.prompt()
			.expect_tty("Failed to read runner ID");

			runners
				.iter()
				.find(|&runner| &runner.name == name)
				.expect(&format!("No runner found with name: `{}`", name))
				.id
		});

	let machine_types = make_request(
		ApiRequest::<ListAllDeploymentMachineTypeRequest>::builder()
			.path(ListAllDeploymentMachineTypePath { workspace_id })
			.query(())
			.headers(ListAllDeploymentMachineTypeRequestHeaders {
				user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
			})
			.body(ListAllDeploymentMachineTypeRequest)
			.build(),
	)
	.await?
	.body
	.machine_types;

	let machine_type = args
		.machine_type
		.map(|machine_type| {
			machine_types
				.iter()
				.find(|mt| mt.id.to_string() == machine_type)
				.expect(&format!(
					"No machine type found with ID: `{}`",
					machine_type
				))
				.id
		})
		.unwrap_or_else(|| {
			let name = Select::new(
				"Select the machine type: ",
				machine_types
					.iter()
					.map(|machine_type| {
						format!(
							"{} vCPU, {} GiB RAM",
							machine_type.cpu_count,
							machine_type.memory_count / 4
						)
					})
					.collect(),
			)
			.with_help_message("The machine type to use for the deployment")
			.prompt()
			.expect_tty("Failed to read machine type");

			machine_types
				.iter()
				.find(|&machine_type| {
					format!(
						"{} vCPU, {} GiB RAM",
						machine_type.cpu_count,
						machine_type.memory_count / 4
					) == name
				})
				.expect(&format!("No machine type found with name: `{}`", name))
				.id
		});

	let deploy_on_push = if registry.is_patr_registry() {
		args.deploy_on_push.unwrap_or_else(|| {
			Confirm::new("Deploy on push?")
				.with_help_message(concat!(
					"If yes, the deployment will automatically update",
					" when a new image is pushed to the registry with the same tag.",
				))
				.with_default(true)
				.prompt()
				.expect_tty("Failed to read deploy on push")
		})
	} else {
		false
	};

	if std::io::stdout().is_terminal() {
		let confirmed = Confirm::new("Create the deployment?")
			.with_default(true)
			.prompt()
			.expect_tty("Failed to read confirmation");

		if !confirmed {
			return Ok(CommandOutput::builder()
				.text("Deployment creation cancelled")
				.json(ApiSuccessResponseBody::empty().to_json_value())
				.build());
		}
	}

	make_request(
		ApiRequest::<CreateDeploymentRequest>::builder()
			.path(CreateDeploymentPath { workspace_id })
			.headers(CreateDeploymentRequestHeaders {
				user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
				authorization: token.clone(),
			})
			.query(())
			.body(CreateDeploymentRequest {
				name,
				registry,
				image_tag,
				runner,
				machine_type,
				running_details: DeploymentRunningDetails {
					deploy_on_push,
					min_horizontal_scale: todo!(),
					max_horizontal_scale: todo!(),
					ports: todo!(),
					environment_variables: todo!(),
					startup_probe: todo!(),
					liveness_probe: todo!(),
					config_mounts: todo!(),
					volumes: todo!(),
				},
				deploy_on_create: todo!(),
			})
			.build(),
	)
	.await?;

	Ok(CommandOutput::builder()
		.text("Deployment created successfully")
		.json(ApiSuccessResponseBody::empty().to_json_value())
		.build())
}
