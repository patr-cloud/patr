use clap::Args as ClapArgs;
use inquire::{Select, Text};
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
				"Please select the deployment to delete:",
				deployments
					.iter()
					.map(|deployment| &deployment.name)
					.collect(),
			)
			.with_formatter(&|deployment| deployment.value.to_string())
			.prompt()
			.expect_tty("Failed to read deployment ID");

			deployments
				.iter()
				.find(|&deployment| &deployment.name == name)
				.expect(&format!("No deployment found with name: `{}`", name))
				.id
		});

	let response = make_request(
		ApiRequest::<DeleteDeploymentRequest>::builder()
			.path(DeleteDeploymentPath {
				workspace_id,
				deployment_id,
			})
			.query(())
			.headers(DeleteDeploymentRequestHeaders {
				authorization: token.clone(),
				user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
			})
			.body(DeleteDeploymentRequest)
			.build(),
	)
	.await?
	.body;

	CommandOutput::builder()
		.text(format!(
			"Deleted deployment `{}` successfully",
			deployment_id
		))
		.json(response.to_json_value())
		.build()
		.into_result()
}
