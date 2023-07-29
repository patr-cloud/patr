use std::io::Write;

use clap::Args;

use super::GlobalArgs;

/// The arguments that can be passed to the switch workspace command.
#[derive(Debug, Clone, Args)]
pub struct SwitchArgs {
	/// Name of the workspace to switch to
	#[arg(short = 'n', long = "name")]
	pub name: String,
}

pub(super) async fn execute(
	global_args: &GlobalArgs,
	args: SwitchArgs,
	mut writer: impl Write + Send,
) -> anyhow::Result<()> {
	Ok(())
}
