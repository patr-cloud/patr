use clap::Args;

use super::GlobalArgs;
use crate::CommandOutput;

/// The arguments that can be passed to the switch workspace command.
#[derive(Debug, Clone, Args)]
pub struct SwitchArgs {
	/// Name of the workspace to switch to
	#[arg(short = 'n', long = "name")]
	pub name: String,
}

pub(super) async fn execute(
	_global_args: &GlobalArgs,
	_args: SwitchArgs,
) -> anyhow::Result<CommandOutput> {
	todo!()
}
