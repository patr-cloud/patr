use clap::Subcommand;

use self::workspace::WorkspaceCommands;
use super::{CommandExecutor, GlobalArgs};
use crate::CommandOutput;

mod infrastructure;
mod workspace;

/// A list of all the commands that can be called on a workspace.
#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum WorkspacedCommands {
	#[command(flatten)]
	WorkspaceCommands(WorkspaceCommands),
	// #[command(flatten)]
	// InfrastructureCommands(InfrastructureCommands),
	// #[command(flatten)]
	// DomainConfigurationCommands(DomainConfigurationCommands),
}

impl CommandExecutor for WorkspacedCommands {
	async fn execute(self, global_args: &GlobalArgs) -> anyhow::Result<CommandOutput> {
		match self {
			Self::WorkspaceCommands(commands) => commands.execute(global_args).await,
			/* Self::InfrastructureCommands(commands) => {
			 * 	commands.execute(global_args, writer).await
			 * }
			 * Self::DomainConfigurationCommands(commands) => {
			 * 	commands.execute(global_args, writer).await
			 * } */
		}
	}
}
