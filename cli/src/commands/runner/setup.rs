use clap::{Args as ClapArgs, ValueEnum};
use models::{ApiErrorResponseBody, utils::False};

use crate::prelude::*;

#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
	/// Force the setup even if the CLI is already configured
	#[arg(short = 'f', long = "force")]
	pub force: bool,
	/// The type of runner to setup
	#[arg(
		value_enum,
		default_value_t = RunnerType::Docker,
		env = "PATR_RUNNER_TYPE"
	)]
	pub runner_type: RunnerType,
}

/// A list of all possible runner types that can be setup.
#[derive(Debug, Copy, Clone, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum RunnerType {
	/// A runner that runs on a local machine and uses Docker to run the
	/// containers
	Docker,
	/// A runner that runs on a Kubernetes cluster and uses the Kubernetes API
	/// to run the containers
	Kubernetes,
}

pub async fn execute(
	args: Args,
	_global_args: GlobalArgs,
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
