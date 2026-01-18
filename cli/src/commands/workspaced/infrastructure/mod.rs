use clap::Subcommand;

use self::{deployment::DeploymentCommand, runner::RunnerCommand};
use crate::prelude::*;

/// All deployment related commands (e.g. list, create, etc.)
mod deployment;
/// All commands to setup / run a runner
mod runner;

/// All infrastructure related commands (e.g. deployments, databases, etc.)
#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum InfrastructureCommand {
	/// All deployment related commands
	#[command(flatten)]
	DeploymentCommand(DeploymentCommand),
	/// All the commands that are related to setting up a runner
	#[command(flatten)]
	RunnerCommands(RunnerCommand),
	// #[command(flatten)]
	// DatabaseCommand(DatabaseCommand),
	// #[command(flatten)]
	// ContainerRegistryCommand(ContainerRegistryCommand),
	// #[command(flatten)]
	// StaticSiteCommand(StaticSiteCommand),
	// #[command(flatten)]
	// SecretCommand(SecretCommand),
}

/// All commands that are executed on workspace related stuff
pub async fn execute(
	command: InfrastructureCommand,
	global_args: GlobalArgs,
	state: AppState,
) -> Result<CommandOutput, AppError> {
	match command {
		InfrastructureCommand::DeploymentCommand(command) => {
			deployment::execute(command, global_args, state).await
		}
		InfrastructureCommand::RunnerCommands(commands) => {
			runner::execute(commands, global_args, state).await
		}
	}
}
