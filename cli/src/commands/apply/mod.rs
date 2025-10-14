use std::collections::BTreeMap;

use clap::Args as ClapArgs;
use inquire::Select;
use models::{
	api::{user::*, workspace::deployment::ExposedPortType},
	iaac::*,
};
use tokio::fs;

use crate::prelude::*;

/// The module to apply a deployment configuration file to the current workspace
mod deployment;

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

	let deserializer = &mut serde_yaml2::de::YamlDeserializer::from_str(&file).unwrap();

	let resources = vec![
		serde_path_to_error::deserialize::<_, IaacResource>(deserializer)
			.map_err(|err| AppError::IaacParseError(format!("{} at `{}`", err, err.path())))?,
	]
	.deduplicated()?;

	for resource in resources {
		// Apply the resource
		match resource.data {
			IaacResourceData::Deployment(deployment) => {
				deployment::apply(workspace_id, token.clone(), deployment).await?;
			}
		}
	}

	CommandOutput::builder()
		.text(format!("File `{}` applied successfully", args.file))
		.json(ApiSuccessResponseBody::empty().to_json_value())
		.build()
		.into_result()
}

#[test]
fn test() {
	println!(
		"{}",
		serde_yaml2::to_string(&OneOrMore::One(IaacResource {
			depends_on: None,
			data: IaacResourceData::Deployment(IaacDeployment {
				id: None,
				name: MaybeExternallySourced::Value("my-deployment".to_string()),
				image: MaybeExternallySourced::Value("grafana/grafana-oss:latest".parse().unwrap()),
				runner: MaybeExternallySourced::Value("Test runner".to_string()),
				machine_type: MaybeExternallySourced::Value("2vCPU 4GB".parse().unwrap()),
				deploy_on_push: MaybeExternallySourced::Value(true),
				ports: IaacDeploymentPorts(BTreeMap::from([(
					StringifiedU16::new(3000),
					ExposedPortType::Http
				)])),
				min_horizontal_scale: MaybeExternallySourced::Value(1),
				max_horizontal_scale: MaybeExternallySourced::Value(1),
				environment_variables: IaacDeploymentEnvVars::default(),
				startup_probe: None,
				liveness_probe: None,
				config_mounts: BTreeMap::new(),
			}),
		}))
		.unwrap()
	);
}
