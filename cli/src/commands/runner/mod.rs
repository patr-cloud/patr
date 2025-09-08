use clap::Subcommand;

use crate::prelude::*;

/// The command to setup the CLI's configuration settings for first time use.
mod setup;
/// Create a new runner
mod create;
/// List all runners
mod list;

/// A list of all the commands that can be called on a workspace.
#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum RunnerCommand {
	/// Setup the CLI's configuration settings for first time use.
	#[command(alias = "configure")]
	Setup(setup::Args),
	/// The command create new runner 
	#[command(alias = "new")]
	Create(create::Args),
	/// The command list all runners
	#[command(alias = "l")]
	List,
}

/// All commands that are executed on workspace related stuff
pub async fn execute(
	command: RunnerCommand,
	global_args: GlobalArgs,
	state: AppState,
) -> Result<CommandOutput, AppError> {
	match command {
		RunnerCommand::Setup(args) => setup::execute(args, global_args, state).await,
		RunnerCommand::Create(args) => create::execute(args, global_args, state).await,
		RunnerCommand::List => list::execute(global_args, state).await
	}
}
