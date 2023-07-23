use std::io::Write;

use clap::Args;

use super::GlobalArgs;

/// The arguments that can be passed to the list workspaces command.
pub type ListArgs = ();

pub(super) async fn execute(
	global_args: &GlobalArgs,
	args: ListArgs,
	mut writer: impl Write + Send,
) -> anyhow::Result<()> {

}
