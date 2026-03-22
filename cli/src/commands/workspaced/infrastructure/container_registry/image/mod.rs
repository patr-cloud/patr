use clap::Subcommand;

use crate::prelude::*;

/// Delete an image (manifest) from a container registry repository
mod delete;
/// List images (manifests) in a container registry repository
mod list;

/// Image (manifest) related commands for a container registry repository
#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum ImageCommand {
	/// List images (manifests) in a repository
	#[command(alias = "ls")]
	List(list::Args),
	/// Delete an image (manifest) from a repository
	#[command(alias = "remove", alias = "rm")]
	Delete(delete::Args),
}

/// Execute an image command
pub async fn execute(
	command: ImageCommand,
	global_args: GlobalArgs,
	state: AppState,
) -> Result<CommandOutput, AppError> {
	match command {
		ImageCommand::List(args) => list::execute(args, global_args, state).await,
		ImageCommand::Delete(args) => delete::execute(args, global_args, state).await,
	}
}
