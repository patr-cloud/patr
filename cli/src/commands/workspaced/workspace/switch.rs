use clap::Args;
use models::ApiErrorResponse;

use crate::prelude::*;

/// The arguments that can be passed to the switch workspace command.
#[derive(Debug, Clone, Args)]
pub struct SwitchArgs {
	/// Name of the workspace to switch to
	#[arg(short = 'n', long = "name")]
	pub name: String,
}

pub(super) async fn execute(
	global_args: GlobalArgs,
	args: SwitchArgs,
	state: AppState,
) -> Result<CommandOutput, ApiErrorResponse> {
	todo!()
}
