use clap::Args;
use models::ApiErrorResponse;

use crate::prelude::*;

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
	global_args: GlobalArgs,
	args: RenameArgs,
	state: AppState,
) -> Result<CommandOutput, ApiErrorResponse> {
	todo!()
}
