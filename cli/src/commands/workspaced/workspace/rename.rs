use clap::Args as ClapArgs;

use crate::prelude::*;

/// The arguments that can be passed to the switch workspace command.
#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
	/// New name of the workspace
	#[arg(short = 'n', long = "name")]
	pub new_name: String,
}

/// The command to rename a workspace
pub(super) async fn execute(
	_global_args: GlobalArgs,
	_args: Args,
	_state: AppState,
) -> Result<CommandOutput, AppError> {
	todo!()
}
