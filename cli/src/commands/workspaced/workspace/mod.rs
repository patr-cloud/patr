use clap::Subcommand;

use self::{create::CreateArgs, rename::RenameArgs, switch::SwitchArgs};
use crate::prelude::*;

mod create;
mod list;
mod rename;
mod switch;

#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum WorkspaceCommand {
	#[command(subcommand, name = "workspace")]
	WorkspaceAction(WorkspaceActionCommand),
	#[command(subcommand)]
	Context(ContextCommands),
	#[command(name = "workspaces")]
	ListWorkspaces,
}

#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum WorkspaceActionCommand {
	Create(CreateArgs),
	Switch(SwitchArgs),
	List,
	Rename(RenameArgs),
}

#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum ContextCommands {
	Switch(SwitchArgs),
}

pub async fn execute(
	command: WorkspaceCommand,
	global_args: GlobalArgs,
	state: AppState,
) -> Result<CommandOutput, AppError> {
	match command {
		WorkspaceCommand::WorkspaceAction(command) => match command {
			WorkspaceActionCommand::Create(args) => create::execute(global_args, args, state).await,
			WorkspaceActionCommand::Switch(args) => switch::execute(global_args, args, state).await,
			WorkspaceActionCommand::List => list::execute(global_args, state).await,
			WorkspaceActionCommand::Rename(args) => rename::execute(global_args, args, state).await,
		},
		WorkspaceCommand::Context(ContextCommands::Switch(args)) => {
			switch::execute(global_args, args, state).await
		}
		WorkspaceCommand::ListWorkspaces => list::execute(global_args, state).await,
	}
}
