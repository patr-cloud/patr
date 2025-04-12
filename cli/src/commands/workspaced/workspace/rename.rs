use clap::Args;

use crate::prelude::*;

/// The arguments that can be passed to the switch workspace command.
#[derive(Debug, Clone, Args)]
pub struct RenameArgs {
	/// New name of the workspace
	#[arg(short = 'n', long = "name")]
	pub new_name: String,
}

/// The command to rename a workspace
pub(super) async fn execute(
	_global_args: GlobalArgs,
	_args: RenameArgs,
	_state: AppState,
) -> Result<CommandOutput, AppError> {
	todo!()
}
