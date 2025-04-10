use clap::Subcommand;

use self::{create::CreateArgs, rename::RenameArgs, switch::SwitchArgs};
use crate::prelude::*;

/// The command to create a new workspace
mod create;
/// The command to list all workspaces that the user is a part of
mod list;
/// The command to rename a workspace
mod rename;
/// The command to switch between workspace contexts
mod switch;

/// All commands that are executed on a workspace
#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum WorkspaceCommand {
	/// The actions that can be performed on a workspace
	#[command(subcommand, name = "workspace")]
	WorkspaceAction(WorkspaceActionCommand),
	/// The command to switch between workspace contexts
	#[command(subcommand)]
	Context(ContextCommand),
	/// The command to list all workspaces that the user is a part of
	#[command(name = "workspaces")]
	ListWorkspaces,
}

/// The actions that can be performed on a workspace
#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum WorkspaceActionCommand {
	/// The command to create a new workspace
	#[command(alias = "new")]
	Create(CreateArgs),
	/// The command to switch between workspace contexts
	Switch(SwitchArgs),
	/// The command to list all workspaces that the user is a part of
	#[command(alias = "ls")]
	List,
	/// The command to rename a workspace
	Rename(RenameArgs),
}

/// The utility command to switch between workspace contexts
#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum ContextCommand {
	/// The command to switch between workspace contexts
	Switch(SwitchArgs),
}

/// The list of all commands that can be executed on a workspace
pub async fn execute(
	command: WorkspaceCommand,
	global_args: GlobalArgs,
	state: AppState,
) -> Result<CommandOutput, AppError> {
	use WorkspaceActionCommand::*;

	match command {
		WorkspaceCommand::WorkspaceAction(Create(args)) => {
			create::execute(global_args, args, state).await
		}
		WorkspaceCommand::WorkspaceAction(Switch(args)) |
		WorkspaceCommand::Context(ContextCommand::Switch(args)) => {
			switch::execute(global_args, args, state).await
		}
		WorkspaceCommand::WorkspaceAction(List) | WorkspaceCommand::ListWorkspaces => {
			list::execute(global_args, state).await
		}
		WorkspaceCommand::WorkspaceAction(Rename(args)) => {
			rename::execute(global_args, args, state).await
		}
	}
}
