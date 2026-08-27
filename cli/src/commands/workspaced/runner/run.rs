use std::path::PathBuf;

use clap::Args as ClapArgs;
use common::prelude::{Runner, RunnerSettings};
use docker::prelude::{DockerRunner, DockerSettings};

use crate::prelude::*;

/// The arguments for the `runner run` command.
#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
	/// Path to the config file (defaults to the standard location)
	#[arg(short = 'c', long = "config")]
	pub config: Option<PathBuf>,
}

/// Run the configured runner in the foreground.
pub async fn execute(args: Args) -> Result<CommandOutput, AppError> {
	let config_path = args.config.unwrap_or_else(crate::utils::runner_config_path);

	let config_str = std::fs::read_to_string(&config_path).map_err(|e| {
		AppError::RunnerError(format!(
			"Failed to read config file at {}: {e}",
			config_path.display()
		))
	})?;

	let config: RunnerSettings<DockerSettings> = serde_json::from_str(&config_str)
		.map_err(|e| AppError::RunnerError(format!("Failed to parse config: {e}")))?;

	let runner = Runner::<DockerRunner>::init_with_config(config)
		.await
		.map_err(|e| AppError::RunnerError(e.to_string()))?;

	runner
		.run()
		.await
		.map_err(|e| AppError::RunnerError(e.to_string()))?
}
