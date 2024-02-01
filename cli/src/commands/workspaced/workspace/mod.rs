use clap::Subcommand;

use self::{create::CreateArgs, rename::RenameArgs, switch::SwitchArgs};
use crate::{
	commands::{CommandExecutor, GlobalArgs},
	CommandOutput,
};

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
	#[command(subcommand, name = "workspaces")]
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
	async fn execute(self, global_args: &GlobalArgs) -> anyhow::Result<CommandOutput> {
		match self {
			Self::WorkspaceAction(commands) => commands.execute(global_args).await,
			Self::Context(ContextCommands::Switch { name }) => {
				WorkspaceActionCommands::Switch(SwitchArgs { name })
					.execute(global_args)
					.await
			}
			Self::ListWorkspaces => WorkspaceActionCommands::List.execute(global_args).await,
		}
	}
}

impl CommandExecutor for WorkspaceActionCommands {
	async fn execute(
		self,
		global_args: &GlobalArgs,
	) -> anyhow::Result<CommandOutput> {
		match self {
			Self::Create(args) => create::execute(global_args, args).await,
			Self::Switch(args) => switch::execute(global_args, args).await,
			Self::List => list::execute(global_args, ()).await,
			Self::Rename(args) => rename::execute(global_args, args).await,
		}
	}
}
