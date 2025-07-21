use clap::Subcommand;

use crate::prelude::*;

/// The command to setup the CLI's configuration settings for first time use.
mod setup;

/// A list of all the commands that can be called on a workspace.
#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum RunnerCommand {
	/// Setup the CLI's configuration settings for first time use.
	#[command(alias = "configure")]
	Setup(setup::Args),
}

/// All commands that are executed on workspace related stuff
pub async fn execute(
	command: RunnerCommand,
	global_args: GlobalArgs,
	state: AppState,
) -> Result<CommandOutput, AppError> {
	match command {
		RunnerCommand::Setup(args) => setup::execute(args, global_args, state).await,
	}
}
