use clap::Args;

use crate::CommandOutput;

use super::GlobalArgs;

/// The arguments that can be passed to the switch workspace command.
#[derive(Debug, Clone, Args)]
pub struct RenameArgs {
	/// Name of the workspace to rename
	#[arg(short = 'w', long = "workspace")]
	pub workspace: String,
	/// New name of the workspace
	#[arg(short = 'n', long = "name")]
	pub new_name: String,
}

pub(super) async fn execute(
	_global_args: &GlobalArgs,
	_args: RenameArgs,
) -> anyhow::Result<CommandOutput> {
	todo!()
}
