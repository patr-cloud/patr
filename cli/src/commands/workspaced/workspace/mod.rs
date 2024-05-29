use clap::Subcommand;
use models::ApiErrorResponse;

use self::{create::CreateArgs, rename::RenameArgs, switch::SwitchArgs};
use crate::prelude::*;

mod create;
mod list;
mod rename;
mod switch;

#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum WorkspaceCommands {
	#[command(subcommand, name = "workspace")]
	WorkspaceAction(WorkspaceActionCommands),
	#[command(subcommand)]
	Context(ContextCommands),
	#[command(name = "workspaces")]
	ListWorkspaces,
}

#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum WorkspaceActionCommands {
	Create(CreateArgs),
	Switch(SwitchArgs),
	List,
	Rename(RenameArgs),
}

#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum ContextCommands {
	Switch { name: String },
}

impl CommandExecutor for WorkspaceCommands {
	async fn execute(
		self,
		global_args: GlobalArgs,
		state: AppState,
	) -> Result<CommandOutput, ApiErrorResponse> {
		match self {
			Self::WorkspaceAction(commands) => commands.execute(global_args, state).await,
			Self::Context(ContextCommands::Switch { name }) => {
				WorkspaceActionCommands::Switch(SwitchArgs { name })
					.execute(global_args, state)
					.await
			}
			Self::ListWorkspaces => {
				WorkspaceActionCommands::List
					.execute(global_args, state)
					.await
			}
		}
	}
}

impl CommandExecutor for WorkspaceActionCommands {
	async fn execute(
		self,
		global_args: GlobalArgs,
		state: AppState,
	) -> Result<CommandOutput, ApiErrorResponse> {
		match self {
			Self::Create(args) => create::execute(global_args, args, state).await,
			Self::Switch(args) => switch::execute(global_args, args, state).await,
			Self::List => list::execute(global_args, (), state).await,
			Self::Rename(args) => rename::execute(global_args, args, state).await,
		}
	}
}
