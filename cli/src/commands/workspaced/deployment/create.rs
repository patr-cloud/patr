use std::{collections::BTreeMap, fmt, io::IsTerminal};

use clap::{ArgAction, Args as ClapArgs};
use inquire::{Confirm, CustomType, Select, Text, validator::Validation};
use models::api::{
	user::*,
	workspace::{container_registry::*, deployment::*, runner::*},
};

use crate::{prelude::*, utils::StringExt};

#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
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
	/// The minimum horizontal scale for the deployment
	#[arg(
		alias = "min",
		alias = "min-scale",
		long = "min-horizontal-scale",
		value_name = "MIN-HORIZONTAL-SCALE",
		env = "PATR_DEPLOYMENT_MIN_HORIZONTAL_SCALE"
	)]
	pub min_horizontal_scale: Option<u16>,
	/// The maximum horizontal scale for the deployment
	#[arg(
		alias = "max",
		alias = "max-scale",
		long = "max-horizontal-scale",
		value_name = "MAX-HORIZONTAL-SCALE",
		env = "PATR_DEPLOYMENT_MAX_HORIZONTAL_SCALE"
	)]
	pub max_horizontal_scale: Option<u16>,
	/// The ports to expose for the deployment. This should be of the format
	/// `PORT:TYPE`, where `TYPE` is one of `http`, `tcp`, or `udp`.
	#[arg(
		short = 'p',
		long = "ports",
		value_name = "PORTS",
		env = "PATR_DEPLOYMENT_PORTS",
		action = ArgAction::Append,
	)]
	pub ports: Option<Vec<String>>,
	/// The environment variables to set for the deployment. This should be of
	/// the format `KEY=VALUE`.
	#[arg(
		short = 'e',
		alias = "env",
		long = "environment",
		value_name = "ENVIRONMENT-VARIABLE",
		action = ArgAction::Append,
	)]
	pub environment_variables: Option<Vec<String>>,
	/// Whether to deploy on create
	#[arg(
		alias = "start",
		alias = "create",
		long = "deploy-on-create",
		env = "PATR_DEPLOYMENT_DEPLOY_ON_CREATE",
		action = ArgAction::SetTrue,
	)]
	pub deploy_on_create: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuField {
	Name,
	Registry,
	ImageTag,
	Runner,
	MachineType,
	DeployOnPush,
	MinScale,
	MaxScale,
	Ports,
	EnvVars,
	DeployOnCreate,
	CreateDeployment,
}

struct MenuItem {
	field: MenuField,
	display: String,
}

impl fmt::Display for MenuItem {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.display)
	}
}

fn build_menu_items(
	name: &Option<String>,
	registry: &Option<DeploymentRegistry>,
	image_tag: &Option<String>,
	runner_name: &Option<String>,
	machine_type_display: &Option<String>,
	deploy_on_push: bool,
	min_horizontal_scale: u16,
	max_horizontal_scale: u16,
	ports: &BTreeMap<StringifiedU16, ExposedPortType>,
	environment_variables: &BTreeMap<String, EnvironmentVariableValue>,
	deploy_on_create: bool,
) -> Vec<MenuItem> {
	let required_remaining = [
		name.is_none(),
		registry.is_none(),
		image_tag.is_none(),
		runner_name.is_none(),
		machine_type_display.is_none(),
	]
	.iter()
	.filter(|&&missing| missing)
	.count();

	let req = |set: bool| if set { " " } else { "* " };

	let registry_display = match registry {
		Some(DeploymentRegistry::PatrRegistry { repository_id, .. }) => {
			format!("patr registry ({})", repository_id)
		}
		Some(DeploymentRegistry::ExternalRegistry {
			registry,
			image_name,
		}) => format!("{}/{}", registry, image_name),
		None => "(not set)".to_string(),
	};

	let ports_display = if ports.is_empty() {
		"(none)".to_string()
	} else {
		ports
			.iter()
			.map(|(port, typ)| format!("{}:{}", port, typ.to_string().to_lowercase()))
			.collect::<Vec<_>>()
			.join(", ")
	};

	let env_display = if environment_variables.is_empty() {
		"(none)".to_string()
	} else {
		format!("{} variable(s)", environment_variables.len())
	};

	let create_label = if required_remaining > 0 {
		format!(
			">> Create Deployment ({} required field{} remaining)",
			required_remaining,
			if required_remaining == 1 { "" } else { "s" }
		)
	} else {
		">> Create Deployment".to_string()
	};

	vec![
		MenuItem {
			field: MenuField::Name,
			display: format!(
				"{}Name: {}",
				req(name.is_some()),
				name.as_deref().unwrap_or("(not set)")
			),
		},
		MenuItem {
			field: MenuField::Registry,
			display: format!("{}Image: {}", req(registry.is_some()), registry_display),
		},
		MenuItem {
			field: MenuField::ImageTag,
			display: format!(
				"{}Image Tag: {}",
				req(image_tag.is_some()),
				image_tag.as_deref().unwrap_or("(not set)")
			),
		},
		MenuItem {
			field: MenuField::Runner,
			display: format!(
				"{}Runner: {}",
				req(runner_name.is_some()),
				runner_name.as_deref().unwrap_or("(not set)")
			),
		},
		MenuItem {
			field: MenuField::MachineType,
			display: format!(
				"{}Machine Type: {}",
				req(machine_type_display.is_some()),
				machine_type_display.as_deref().unwrap_or("(not set)")
			),
		},
		MenuItem {
			field: MenuField::DeployOnPush,
			display: format!(" Deploy on Push: {}", deploy_on_push),
		},
		MenuItem {
			field: MenuField::MinScale,
			display: format!(" Min Scale: {}", min_horizontal_scale),
		},
		MenuItem {
			field: MenuField::MaxScale,
			display: format!(" Max Scale: {}", max_horizontal_scale),
		},
		MenuItem {
			field: MenuField::Ports,
			display: format!(" Ports: {}", ports_display),
		},
		MenuItem {
			field: MenuField::EnvVars,
			display: format!(" Env Variables: {}", env_display),
		},
		MenuItem {
			field: MenuField::DeployOnCreate,
			display: format!(" Deploy on Create: {}", deploy_on_create),
		},
		MenuItem {
			field: MenuField::CreateDeployment,
			display: create_label,
		},
	]
}

fn parse_ports_from_args(
	ports: Vec<String>,
) -> Result<BTreeMap<StringifiedU16, ExposedPortType>, AppError> {
	ports
		.into_iter()
		.map(|port| {
			let Some((port_number, port_type)) = port.split_once(':') else {
				return Err(AppError::ParseError(format!(
					"Invalid port format: `{}`. Expected format is `PORT:TYPE`.",
					port
				)));
			};

			let port_number = port_number
				.parse::<u16>()
				.map_err(|_| {
					AppError::ParseError(format!("Invalid port number: `{}`", port_number))
				})?
				.into();

			let port_type = match port_type.to_lowercase().as_str() {
				"http" => ExposedPortType::Http,
				"tcp" => ExposedPortType::Tcp,
				"udp" => ExposedPortType::Udp,
				_ => {
					return Err(AppError::ParseError(format!(
						"Invalid port type: `{}`. Expected one of `http`, `tcp`, or `udp`.",
						port_type
					)));
				}
			};

			Ok((port_number, port_type))
		})
		.collect()
}

fn parse_env_vars_from_args(
	env_vars: Vec<String>,
) -> Result<BTreeMap<String, EnvironmentVariableValue>, AppError> {
	env_vars
		.into_iter()
		.map(|env_var| {
			let Some((key, value)) = env_var.split_once('=') else {
				return Err(AppError::ParseError(format!(
					"Invalid environment variable format: `{}`. Expected format is `KEY=VALUE`.",
					env_var
				)));
			};

			if key.is_empty() || value.is_empty() {
				return Err(AppError::ParseError(format!(
					"Environment variable key or value cannot be empty: `{}`",
					env_var
				)));
			}

			Ok((
				key.to_string(),
				EnvironmentVariableValue::String(value.to_string()),
			))
		})
		.collect()
}

async fn search_runners(
	workspace_id: Uuid,
	token: &BearerToken,
	name_query: &str,
) -> Result<Vec<WithId<Runner>>, AppError> {
	let search = if name_query.is_empty() {
		Default::default()
	} else {
		RunnerSearchParams {
			name: Some(name_query.to_string()),
			..Default::default()
		}
	};

	Ok(make_request(
		ApiRequest::<ListRunnersForWorkspaceRequest>::builder()
			.path(ListRunnersForWorkspacePath { workspace_id })
			.query(ListResourceQuery {
				page: 0,
				count: ListResourceQuery::DEFAULT_PAGE_SIZE,
				search,
				sort: Default::default(),
				additional_query: (),
			})
			.headers(ListRunnersForWorkspaceRequestHeaders {
				user_agent: constants::USER_AGENT,
				authorization: token.clone(),
			})
			.build(),
	)
	.await?
	.body
	.runners)
}

async fn search_repositories(
	workspace_id: Uuid,
	token: &BearerToken,
	name_query: &str,
) -> Result<Vec<WithId<ContainerRepository>>, AppError> {
	let search = if name_query.is_empty() {
		Default::default()
	} else {
		ContainerRepositorySearchParams {
			name: Some(name_query.to_string()),
			..Default::default()
		}
	};

	Ok(make_request(
		ApiRequest::<ListContainerRepositoriesRequest>::builder()
			.path(ListContainerRepositoriesPath { workspace_id })
			.headers(ListContainerRepositoriesRequestHeaders {
				authorization: token.clone(),
				user_agent: constants::USER_AGENT,
			})
			.query(ListResourceQuery {
				page: 0,
				count: ListResourceQuery::DEFAULT_PAGE_SIZE,
				search,
				sort: Default::default(),
				additional_query: (),
			})
			.build(),
	)
	.await?
	.body
	.repositories)
}

async fn edit_registry(
	workspace_id: Uuid,
	token: &BearerToken,
	current_registry: &mut Option<DeploymentRegistry>,
) -> Result<(), AppError> {
	let registry_name = Text::new("Registry:")
		.with_autocomplete(|text: &str| {
			let results = vec!["registry.patr.cloud".to_string(), "docker.io".to_string()]
				.into_iter()
				.filter(|name| name.to_lowercase().contains(&text.to_lowercase()))
				.collect();
			Ok(results)
		})
		.prompt_skippable()
		.expect_tty("Failed to read registry name");

	let Some(registry_name) = registry_name else {
		return Ok(());
	};

	if registry_name == "registry.patr.cloud" {
		let result = SearchAndSelect::new(
			"Search repositories:",
			|query| {
				let token = token.clone();
				let query = query.to_owned();
				async move { search_repositories(workspace_id, &token, &query).await }
			},
			|r| r.name.clone(),
		)
		.with_help_message("Type to filter by name, or press Enter to list all")
		.prompt_skippable()
		.await?;

		let Some(repo) = result else {
			return Ok(());
		};

		*current_registry = Some(DeploymentRegistry::PatrRegistry {
			registry: PatrRegistry,
			repository_id: repo.id,
		});
	} else {
		let image_name = Text::new("Image name:")
			.prompt_skippable()
			.expect_tty("Failed to read image name");

		let Some(image_name) = image_name else {
			return Ok(());
		};

		*current_registry = Some(DeploymentRegistry::ExternalRegistry {
			registry: registry_name,
			image_name,
		});
	}

	Ok(())
}

fn edit_ports(ports: &mut BTreeMap<StringifiedU16, ExposedPortType>) {
	loop {
		let mut choices = vec!["Add port".to_string()];
		let existing: Vec<String> = ports
			.iter()
			.map(|(port, typ)| format!("{}/{}", port, typ.to_string().to_lowercase()))
			.collect();
		for p in &existing {
			choices.push(format!("Remove {}", p));
		}
		choices.push("Done".to_string());

		let Some(selection) = Select::new("Ports:", choices)
			.prompt_skippable()
			.expect_tty("Failed to read port action")
		else {
			return;
		};

		if selection == "Done" {
			return;
		} else if selection == "Add port" {
			let Some(port) = CustomType::<u16>::new("Port number:")
				.with_validator(|input: &u16| {
					if *input == 0 {
						Ok(Validation::Invalid(inquire::validator::ErrorMessage::from(
							"Port must be > 0",
						)))
					} else {
						Ok(Validation::Valid)
					}
				})
				.prompt_skippable()
				.expect_tty("Failed to read port")
			else {
				continue;
			};

			let Some(port_type) = Select::new(
				"Port type:",
				vec![
					ExposedPortType::Http,
					ExposedPortType::Tcp,
					ExposedPortType::Udp,
				],
			)
			.with_formatter(&|input| input.value.to_string().to_lowercase())
			.prompt_skippable()
			.expect_tty("Failed to read port type") else {
				continue;
			};

			ports.insert(port.into(), port_type);
		} else if let Some(remove_label) = selection.strip_prefix("Remove ") {
			let port_str = remove_label.split(':').next().unwrap_or("");
			if let Ok(port_num) = port_str.parse::<u16>() {
				ports.remove(&StringifiedU16::from(port_num));
			}
		}
	}
}

fn edit_env_vars(env_vars: &mut BTreeMap<String, EnvironmentVariableValue>) {
	loop {
		let mut choices = vec!["Add variable".to_string()];
		for key in env_vars.keys() {
			choices.push(format!("Remove {}", key));
		}
		choices.push("Done".to_string());

		let Some(selection) = Select::new("Environment variables:", choices)
			.prompt_skippable()
			.expect_tty("Failed to read env var action")
		else {
			return;
		};

		if selection == "Done" {
			return;
		} else if selection == "Add variable" {
			let Some(key) = Text::new("Key:")
				.prompt_skippable()
				.expect_tty("Failed to read env var key")
			else {
				continue;
			};

			if key.is_empty() {
				continue;
			}

			let Some(value) = Text::new("Value:")
				.prompt_skippable()
				.expect_tty("Failed to read env var value")
			else {
				continue;
			};

			env_vars.insert(key, EnvironmentVariableValue::String(value));
		} else if let Some(key) = selection.strip_prefix("Remove ") {
			env_vars.remove(key);
		}
	}
}

pub async fn execute(
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

	// Pre-populate from CLI args
	let mut name = args.name;
	let mut image_tag = args.tag;
	let mut deploy_on_push = args.deploy_on_push.unwrap_or(false);
	let mut min_horizontal_scale = args.min_horizontal_scale.unwrap_or(1);
	let mut max_horizontal_scale = args.max_horizontal_scale.unwrap_or(1);
	let mut deploy_on_create = args.deploy_on_create.unwrap_or(true);

	let mut ports = args
		.ports
		.map(parse_ports_from_args)
		.transpose()?
		.unwrap_or_default();

	let mut environment_variables = args
		.environment_variables
		.map(parse_env_vars_from_args)
		.transpose()?
		.unwrap_or_default();

	// Resolve runner from args if provided
	let mut runner = None;
	let mut runner_name = None;
	if let Some(runner_arg) = &args.runner {
		// Try parsing as UUID first, otherwise search by name
		let runners = search_runners(workspace_id, &token, runner_arg).await?;
		let id = Uuid::parse_str(runner_arg).ok();
		if let Some(r) = runners
			.iter()
			.find(|r| r.name == *runner_arg || id.filter(|id| r.id == *id).is_some())
		{
			runner = Some(r.id);
			runner_name = Some(r.name.clone());
		} else {
			return Err(AppError::ParseError(format!(
				"No runner found with ID or name: `{}`",
				runner_arg
			)));
		}
	}

	// Resolve registry from args if provided
	let mut registry = None;
	if let Some(registry_arg) = &args.registry {
		if registry_arg == "registry.patr.cloud" {
			if let Some(image_arg) = &args.image {
				let repositories = search_repositories(workspace_id, &token, image_arg).await?;
				let id = Uuid::parse_str(image_arg).ok();
				if let Some(repo) = repositories
					.iter()
					.find(|r| r.name == *image_arg || id.filter(|id| r.id == *id).is_some())
				{
					registry = Some(DeploymentRegistry::PatrRegistry {
						registry: PatrRegistry,
						repository_id: repo.id,
					});
				} else {
					return Err(AppError::ParseError(format!(
						"No repository found with name or ID: `{}`",
						image_arg
					)));
				}
			}
		} else if let Some(image_arg) = &args.image {
			registry = Some(DeploymentRegistry::ExternalRegistry {
				registry: registry_arg.clone(),
				image_name: image_arg.clone(),
			});
		}
	}

	// Resolve machine type from args if provided
	let mut machine_type = None;
	let mut machine_type_display = None;
	if let Some(mt_arg) = &args.machine_type {
		let machine_types = make_request(
			ApiRequest::<ListAllDeploymentMachineTypeRequest>::builder()
				.path(ListAllDeploymentMachineTypePath { workspace_id })
				.headers(ListAllDeploymentMachineTypeRequestHeaders {
					user_agent: constants::USER_AGENT,
				})
				.build(),
		)
		.await?
		.body
		.machine_types;

		if let Some(mt) = machine_types.iter().find(|mt| mt.id.to_string() == *mt_arg) {
			machine_type = Some(mt.id);
			machine_type_display = Some(format!(
				"{} vCPU, {} GiB RAM",
				mt.cpu_count,
				mt.memory_count / 4
			));
		} else {
			return Err(AppError::ParseError(format!(
				"No machine type found with ID: `{}`",
				mt_arg
			)));
		}
	}

	macro_rules! is_ready {
		() => {
			name.is_some() &&
				registry.is_some() &&
				image_tag.is_some() &&
				runner.is_some() &&
				machine_type.is_some()
		};
	}

	// Interactive menu loop
	loop {
		if !std::io::stdout().is_terminal() && is_ready!() {
			break;
		}
		clear_screen();

		let items = build_menu_items(
			&name,
			&registry,
			&image_tag,
			&runner_name,
			&machine_type_display,
			deploy_on_push,
			min_horizontal_scale,
			max_horizontal_scale,
			&ports,
			&environment_variables,
			deploy_on_create,
		);

		let page_size = crossterm::terminal::size()
			.map(|(_cols, rows)| rows.saturating_sub(2) as usize)
			.unwrap_or(7)
			.min(items.len())
			.max(7);
		let selection = Select::new("Configure deployment:", items)
			.with_page_size(page_size)
			.prompt()
			.expect_tty("Failed to read menu selection");

		match selection.field {
			MenuField::Name => {
				let mut prompt = Text::new("Deployment name:");
				if let Some(current) = &name {
					prompt = prompt.with_default(current);
				}
				if let Some(new_name) = prompt
					.prompt_skippable()
					.expect_tty("Failed to read deployment name")
				{
					let new_name = new_name.some_if_not_empty();
					if new_name.is_some() {
						name = new_name;
					}
				}
			}
			MenuField::Registry => {
				edit_registry(workspace_id, &token, &mut registry).await?;
			}
			MenuField::ImageTag => {
				let default = image_tag.as_deref().unwrap_or("latest");
				if let Some(new_tag) = Text::new("Image tag:")
					.with_default(default)
					.with_help_message("The tag of the image to use (eg: `latest`, `stable`, etc)")
					.prompt_skippable()
					.expect_tty("Failed to read image tag")
				{
					let tag = new_tag
						.some_if_not_empty()
						.unwrap_or_else(|| "latest".to_string());
					image_tag = Some(tag);
				}
			}
			MenuField::Runner => {
				let result = SearchAndSelect::new(
					"Search runners:",
					|query| {
						let token = token.clone();
						let query = query.to_owned();
						async move { search_runners(workspace_id, &token, &query).await }
					},
					|r| r.name.clone(),
				)
				.with_help_message("Type to filter runners by name, or press Enter to list all")
				.prompt_skippable()
				.await?;

				if let Some(r) = result {
					runner_name = Some(r.name.clone());
					runner = Some(r.id);
				}
			}
			MenuField::MachineType => {
				let machine_types = make_request(
					ApiRequest::<ListAllDeploymentMachineTypeRequest>::builder()
						.path(ListAllDeploymentMachineTypePath { workspace_id })
						.headers(ListAllDeploymentMachineTypeRequestHeaders {
							user_agent: constants::USER_AGENT,
						})
						.build(),
				)
				.await?
				.body
				.machine_types;

				let display_names: Vec<String> = machine_types
					.iter()
					.map(|mt| format!("{} vCPU, {} GiB RAM", mt.cpu_count, mt.memory_count / 4))
					.collect();

				if let Some(selected) = Select::new("Select machine type:", display_names.clone())
					.with_help_message("The machine type to use for the deployment")
					.prompt_skippable()
					.expect_tty("Failed to read machine type")
				{
					let idx = display_names
						.iter()
						.position(|d| *d == selected)
						.expect("Selected machine type not found");
					machine_type = Some(machine_types[idx].id);
					machine_type_display = Some(selected);
				}
			}
			MenuField::DeployOnPush => {
				if registry.as_ref().is_some_and(|r| r.is_patr_registry()) {
					if let Some(val) = Confirm::new("Deploy on push?")
						.with_help_message(concat!(
							"If yes, the deployment will automatically update",
							" when a new image is pushed to the registry with the same tag.",
						))
						.with_default(deploy_on_push)
						.prompt_skippable()
						.expect_tty("Failed to read deploy on push")
					{
						deploy_on_push = val;
					}
				} else {
					eprintln!("Deploy on push is only available for Patr registry deployments.");
				}
			}
			MenuField::MinScale => {
				if let Some(val) = CustomType::<u16>::new("Minimum horizontal scale:")
					.with_help_message("The minimum number of instances to run for the deployment")
					.with_default(min_horizontal_scale)
					.with_validator(|input: &u16| {
						if *input < 1 {
							Ok(Validation::Invalid(inquire::validator::ErrorMessage::from(
								"Minimum horizontal scale must be at least 1",
							)))
						} else {
							Ok(Validation::Valid)
						}
					})
					.prompt_skippable()
					.expect_tty("Failed to read minimum horizontal scale")
				{
					min_horizontal_scale = val;
					if max_horizontal_scale < min_horizontal_scale {
						max_horizontal_scale = min_horizontal_scale;
					}
				}
			}
			MenuField::MaxScale => {
				let min = min_horizontal_scale;
				if let Some(val) = CustomType::<u16>::new("Maximum horizontal scale:")
					.with_help_message("The maximum number of instances to run for the deployment")
					.with_default(max_horizontal_scale)
					.with_validator(move |input: &u16| {
						if *input < min {
							Ok(Validation::Invalid(inquire::validator::ErrorMessage::from(
								"Maximum horizontal scale must be at least the minimum",
							)))
						} else {
							Ok(Validation::Valid)
						}
					})
					.prompt_skippable()
					.expect_tty("Failed to read maximum horizontal scale")
				{
					max_horizontal_scale = val;
				}
			}
			MenuField::Ports => {
				edit_ports(&mut ports);
			}
			MenuField::EnvVars => {
				edit_env_vars(&mut environment_variables);
			}
			MenuField::DeployOnCreate => {
				if let Some(val) = Confirm::new("Deploy on create?")
					.with_help_message(concat!(
						"If yes, the deployment will be created and deployed immediately.",
						" If no, the deployment will be created but not deployed.",
					))
					.with_default(deploy_on_create)
					.prompt_skippable()
					.expect_tty("Failed to read deploy on create")
				{
					deploy_on_create = val;
				}
			}
			MenuField::CreateDeployment => {
				if !is_ready!() {
					eprintln!("Please fill in all required fields first.");
					continue;
				}
				break;
			}
		}
	}

	make_request(
		ApiRequest::<CreateDeploymentRequest>::builder()
			.path(CreateDeploymentPath { workspace_id })
			.headers(CreateDeploymentRequestHeaders {
				user_agent: constants::USER_AGENT,
				authorization: token.clone(),
			})
			.body(CreateDeploymentRequest {
				name: name.unwrap(),
				registry: registry.unwrap(),
				image_tag: image_tag.unwrap(),
				runner: runner.unwrap(),
				machine_type: machine_type.unwrap(),
				running_details: DeploymentRunningDetails {
					deploy_on_push,
					min_horizontal_scale,
					max_horizontal_scale,
					ports,
					environment_variables,
					startup_probe: None,
					liveness_probe: None,
					config_mounts: BTreeMap::new(),
					volumes: BTreeMap::new(),
				},
				deploy_on_create,
			})
			.build(),
	)
	.await?;

	CommandOutput::builder()
		.text("Deployment created successfully")
		.json(ApiSuccessResponseBody::empty().to_json_value())
		.build()
		.into_result()
}
