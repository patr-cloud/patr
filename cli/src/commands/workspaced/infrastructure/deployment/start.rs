use clap::Args as ClapArgs;
use inquire::Select;
use models::api::{user::*, workspace::deployment::*};

use crate::prelude::*;

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
	/// If set, forces the deployment to restart even if it is already running
	#[arg(short = 'f', long = "force-restart", default_value_t = false)]
	pub force_restart: bool,
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

	let mut deployments = vec![];
	let mut start = 0;

	loop {
		let response = make_request(
			ApiRequest::<ListDeploymentRequest>::builder()
				.path(ListDeploymentPath { workspace_id })
				.headers(ListDeploymentRequestHeaders {
					authorization: token.clone(),
					user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
				})
				.query(Paginated {
					page: start / Paginated::DEFAULT_PAGE_SIZE,
					count: Paginated::DEFAULT_PAGE_SIZE,
					data: (),
				})
				.body(ListDeploymentRequest)
				.build(),
		)
		.await?;

		start += response.body.deployments.len();

		deployments.extend(response.body.deployments);

		if start >= response.headers.total_count.0 {
			break;
		}
	}

	let deployment_id = args
		.name
		.and_then(|name| {
			let id = Uuid::parse_str(&name).ok();
			deployments
				.iter()
				.find(|r| r.name == name || id.filter(|id| r.id == *id).is_some())
				.map(|deployment| deployment.id)
		})
		.unwrap_or_else(|| {
			let name = Select::new(
				"Please select the deployment to start:",
				deployments
					.iter()
					.map(|deployment| &deployment.name)
					.collect(),
			)
			.prompt()
			.expect_tty("Failed to read deployment ID");

			deployments
				.iter()
				.find(|&deployment| &deployment.name == name)
				.expect(&format!("No deployment found with name: `{}`", name))
				.id
		});

	let response = make_request(
		ApiRequest::<StartDeploymentRequest>::builder()
			.path(StartDeploymentPath {
				workspace_id,
				deployment_id,
			})
			.query(StartDeploymentQuery {
				force_restart: args.force_restart,
			})
			.headers(StartDeploymentRequestHeaders {
				authorization: token.clone(),
				user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
			})
			.body(StartDeploymentRequest)
			.build(),
	)
	.await?
	.body;

	CommandOutput::builder()
		.text(format!(
			"Started deployment `{}` successfully",
			deployment_id
		))
		.json(response.to_json_value())
		.build()
		.into_result()
}
