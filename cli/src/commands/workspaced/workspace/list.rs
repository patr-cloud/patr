use std::io::Write;

use super::GlobalArgs;

pub(super) async fn execute(
	global_args: &GlobalArgs,
	_: (),
	mut writer: impl Write + Send,
) -> anyhow::Result<()> {
	Ok(())
}
