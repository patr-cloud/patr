use std::io::Write;

use clap::Args;

use super::GlobalArgs;

/// The arguments that can be passed to the create workspace command.
#[derive(Debug, Clone, Args)]
pub struct CreateArgs {
    /// Name of the workspace to be created
	#[arg(short = 'n', long = "name")]
    pub name: String,
}

pub(super) async fn execute(
	global_args: &GlobalArgs,
	args: CreateArgs,
	mut writer: impl Write + Send,
) -> anyhow::Result<()> {
	Ok(())
}
