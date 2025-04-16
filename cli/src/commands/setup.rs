use clap::Args;
use models::{ApiErrorResponseBody, utils::False};

use crate::prelude::*;

#[derive(Debug, Clone, Args)]
pub struct SetupArgs {
	/// Force the setup even if the CLI is already configured
	#[arg(short = 'f', long = "force")]
	pub force: bool,
}

pub async fn execute(
	args: SetupArgs,
	global_args: GlobalArgs,
	_: AppState,
) -> Result<CommandOutput, AppError> {
	let state = AppState::load();

	if state.is_ok() && !args.force {
		let message = concat!(
			"The CLI already has a configuration setup. ",
			"To override it, use the `--force` flag."
		);
		return CommandOutput::builder()
			.text(message)
			.json(
				ApiErrorResponseBody {
					success: False,
					error: ErrorType::ResourceAlreadyExists,
					message: message.to_string(),
				}
				.to_json_value(),
			)
			.build()
			.into_result();
	}

	todo!()
}
