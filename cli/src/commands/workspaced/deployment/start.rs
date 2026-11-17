use clap::Args as ClapArgs;
use inquire::Select;
use models::api::{
	user::*,
	workspace::{deployment::*, runner::*},
};

use crate::prelude::*;

/// Args for `patr deployment start`.
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

/// Start a deployment (by name/id, or picked interactively).
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

	let mut deployments = vec![];
	let mut start = 0;

	loop {
		let response = make_request(
			ApiRequest::<ListDeploymentRequest>::builder()
				.path(ListDeploymentPath { workspace_id })
				.headers(ListDeploymentRequestHeaders {
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

		start += response.body.deployments.len();

		deployments.extend(response.body.deployments);

		if start >= response.headers.total_count.0 {
			break;
		}
	}

	let deployment = args
		.name
		.and_then(|name| {
			let id = Uuid::parse_str(&name).ok();
			deployments
				.iter()
				.find(|r| r.name == name || id.filter(|id| r.id == *id).is_some())
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
				.unwrap_or_else(|| panic!("No deployment found with name: `{}`", name))
		});

	let runner = match make_request(
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
	.await
	.map_err(AppError::from)
	{
		Ok(response) => Some(response.body.runner.data),
		Err(AppError::ApiError(ErrorType::Unauthorized)) => None,
		Err(err) => return Err(err),
	};

	let response = make_request(
		ApiRequest::<StartDeploymentRequest>::builder()
			.path(StartDeploymentPath {
				workspace_id,
				deployment_id: deployment.id,
			})
			.query(StartDeploymentQuery {
				force_restart: args.force_restart,
			})
			.headers(StartDeploymentRequestHeaders {
				authorization: token.clone(),
				user_agent: constants::USER_AGENT,
			})
			.build(),
	)
	.await?
	.body;

	CommandOutput::builder()
		.text(match runner {
			Some(runner) if runner.connected => format!(
				"Start requested. The runner \"{}\" will deploy \"{}\".",
				runner.name, deployment.name
			),
			Some(runner) => format!(
				concat!(
					"Start requested. The runner \"{}\" is offline, so \"{}\" ",
					"will start once it reconnects.",
				),
				runner.name, deployment.name
			),
			None => "Start requested.".to_string(),
		})
		.json(response.to_json_value())
		.build()
		.into_result()
}
