use clap::Subcommand;

use self::{
	container_registry::ContainerRegistryCommand,
	deployment::DeploymentCommand,
	runner::RunnerCommand,
	workspace::WorkspaceCommand,
};
use crate::prelude::*;

/// All container registry related commands
mod container_registry;
/// All deployment related commands (e.g. list, create, etc.)
mod deployment;
/// All commands to setup / run a runner
mod runner;
/// All the commands that can be executed on a workspace.
mod workspace;

/// A list of all the commands that can be called on a workspace.
#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum WorkspacedCommand {
	/// All commands that can be executed on a workspace
	#[command(flatten)]
	WorkspaceCommands(WorkspaceCommand),
	/// All deployment related commands
	#[command(flatten)]
	DeploymentCommand(DeploymentCommand),
	/// All the commands that are related to setting up a runner
	#[command(flatten)]
	RunnerCommands(RunnerCommand),
	/// All container registry related commands
	#[command(flatten)]
	ContainerRegistryCommand(ContainerRegistryCommand),
}

/// All commands that are executed on workspace related stuff
pub async fn execute(
	command: WorkspacedCommand,
	global_args: GlobalArgs,
	state: AppState,
) -> Result<CommandOutput, AppError> {
	match command {
		WorkspacedCommand::WorkspaceCommands(commands) => {
			workspace::execute(commands, global_args, state).await
		}
		WorkspacedCommand::DeploymentCommand(command) => {
			deployment::execute(command, global_args, state).await
		}
		WorkspacedCommand::RunnerCommands(commands) => {
			runner::execute(commands, global_args, state).await
		}
		WorkspacedCommand::ContainerRegistryCommand(command) => {
			container_registry::execute(command, global_args, state).await
		}
	}
}
