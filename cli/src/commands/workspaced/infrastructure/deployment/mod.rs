use clap::Subcommand;

use self::{create::CreateArgs, start::StartArgs};
use crate::prelude::*;

/// Create a new deployment
mod create;
/// List all deployments in the workspace
mod list;
/// Start a deployment
mod start;

/// All infrastructure related commands (e.g. deployments, databases, etc.)
#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum DeploymentCommand {
	/// The commands that can be executed on deployments
	#[command(subcommand, name = "deployment")]
	DeploymentAction(DeploymentActionCommand),
	/// The command to list all deployments in the workspace
	#[command(name = "deployments")]
	ListDeployments,
}

/// The actions that can be performed on a deployment
#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum DeploymentActionCommand {
	/// The command to list all deployments in the workspace
	#[command(name = "list")]
	List,
	/// The command to create a new deployment
	#[command(name = "new", alias = "create", alias = "add", alias = "init")]
	Create(CreateArgs),
	/// The command to start a deployment
	#[command(name = "start", alias = "run", alias = "up")]
	Start(StartArgs),
}

pub async fn execute(
	command: DeploymentCommand,
	global_args: GlobalArgs,
	state: AppState,
) -> Result<CommandOutput, AppError> {
	use DeploymentActionCommand::*;

	match command {
		DeploymentCommand::DeploymentAction(List) | DeploymentCommand::ListDeployments => {
			list::execute(global_args, state).await
		}
		DeploymentCommand::DeploymentAction(Create(args)) => {
			create::execute(args, global_args, state).await
		}
		DeploymentCommand::DeploymentAction(Start(args)) => {
			start::execute(args, global_args, state).await
		}
	}
}
