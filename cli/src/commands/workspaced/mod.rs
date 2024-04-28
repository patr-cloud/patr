use clap::Subcommand;
use models::ApiErrorResponse;

use self::workspace::WorkspaceCommands;
use super::{CommandExecutor, GlobalArgs};
use crate::prelude::*;

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
	async fn execute(
		self,
		global_args: GlobalArgs,
		state: AppState,
	) -> Result<CommandOutput, ApiErrorResponse> {
		match self {
			Self::WorkspaceCommands(commands) => commands.execute(global_args, state).await,
			/* Self::InfrastructureCommands(commands) => {
			 * 	commands.execute(global_args, writer).await
			 * }
			 * Self::DomainConfigurationCommands(commands) => {
			 * 	commands.execute(global_args, writer).await
			 * } */
		}
	}
}
