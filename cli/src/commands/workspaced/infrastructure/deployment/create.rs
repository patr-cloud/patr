use std::io::IsTerminal;

use clap::Args;
use inquire::{Confirm, Select, Text};
use models::api::{
	user::*,
	workspace::{container_registry::*, deployment::*},
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
	/// The environment to use for the deployment
	#[arg(short = 'e', long = "environment", value_name = "ENVIRONMENT")]
	pub environment: Option<Vec<String>>,
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
				Select::new(
					"Please select the repository to use:",
					repositories.iter().map(|repo| repo.id).collect(),
				)
				.with_formatter(&|repo_id| {
					repositories
						.get(repo_id.index)
						.map(|repo| format!("registry.patr.cloud/{workspace_id}/{}", repo.name))
						.unwrap_or_else(|| repo_id.to_string())
				})
				.prompt()
				.expect_tty("Failed to read repository ID")
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
				runner: todo!(),
				machine_type: todo!(),
				running_details: DeploymentRunningDetails {
					deploy_on_push: todo!(),
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
