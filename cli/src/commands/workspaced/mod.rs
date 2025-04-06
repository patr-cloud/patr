use clap::Subcommand;

use self::workspace::WorkspaceCommand;
use super::GlobalArgs;
use crate::prelude::*;

mod infrastructure;
mod workspace;

/// A list of all the commands that can be called on a workspace.
#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum WorkspacedCommand {
	#[command(flatten)]
	WorkspaceCommands(WorkspaceCommand),
	// #[command(flatten)]
	// InfrastructureCommands(InfrastructureCommands),
	// #[command(flatten)]
	// DomainConfigurationCommands(DomainConfigurationCommands),
}

pub async fn execute(
	command: WorkspacedCommand,
	global_args: GlobalArgs,
	state: AppState,
) -> Result<CommandOutput, AppError> {
	match command {
		WorkspacedCommand::WorkspaceCommands(commands) => {
			workspace::execute(commands, global_args, state).await
		} /* Self::InfrastructureCommands(commands) => {
		   * 	commands.execute(global_args, writer).await
		   * }
		   * Self::DomainConfigurationCommands(commands) => {
		   * 	commands.execute(global_args, writer).await
		   * } */
	}
}
