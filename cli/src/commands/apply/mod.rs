use clap::Args as ClapArgs;
use inquire::Select;
use models::{api::user::*, iaac::*};
use serde_yaml2::de::YamlDeserializer;
use tokio::fs;

use crate::prelude::*;

/// The module to apply a deployment configuration file to the current workspace
mod deployment;
/// The module to apply a domain configuration file to the current workspace
mod domain;
/// The module to apply a managed URL configuration file to the current
/// workspace
mod managed_url;

#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
	/// The filename of the configuration file to apply
	#[arg(short = 'f', long = "file", value_name = "FILE")]
	pub file: String,
	/// Dry run the apply command without making any changes
	#[arg(
		short = 't',
		long = "test",
		alias = "d",
		alias = "dry-run",
		default_value_t = false
	)]
	pub dry_run: bool,
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
				.path(ListUserWorkspacesPath)
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

	let file = fs::read_to_string(&args.file)
		.await
		.map_err(|err| AppError::IaacParseError(err.to_string()))?;
	let resources = serde_path_to_error::deserialize::<_, Vec<IaacResource>>(
		&mut YamlDeserializer::from_str(&file).unwrap(),
	)
	.map_err(|err| AppError::IaacParseError(err.to_string()))?;

	for resource in resources {
		// Apply the resource
		match resource.data {
			IaacResourceData::Deployment(deployment) => {
				deployment::apply(workspace_id, token.clone(), deployment).await?;
			}
			IaacResourceData::Domain(domain) => {
				domain::apply(workspace_id, token.clone(), domain).await?;
			}
			IaacResourceData::ManagedUrl(managed_url) => {
				managed_url::apply(workspace_id, token.clone(), managed_url).await?;
			}
		}
	}

	CommandOutput::builder()
		.text(format!("File `{}` applied successfully", args.file))
		.json(ApiSuccessResponseBody::empty().to_json_value())
		.build()
		.into_result()
}
