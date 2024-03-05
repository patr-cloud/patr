use clap::Args;

use super::GlobalArgs;
use crate::CommandOutput;

/// The arguments that can be passed to the create workspace command.
#[derive(Debug, Clone, Args)]
pub struct CreateArgs {
	/// Name of the workspace to be created
	#[arg(short = 'n', long = "name")]
	pub name: String,
}

pub(super) async fn execute(
	_global_args: &GlobalArgs,
	_args: CreateArgs,
) -> anyhow::Result<CommandOutput> {
	todo!()
}
