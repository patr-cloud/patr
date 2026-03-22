use std::path::PathBuf;

use clap::Args as ClapArgs;
use common::prelude::{Runner, RunnerSettings};
use docker::prelude::{DockerRunner, DockerSettings};

use crate::prelude::*;

/// The arguments for the `runner run` command.
#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
	/// The type of runner to run
	#[arg(value_enum)]
	pub runner_type: RunnerType,
	/// Path to the config file (defaults to standard location for the runner
	/// type)
	#[arg(short = 'c', long = "config")]
	pub config: Option<PathBuf>,
}

pub async fn execute(args: Args) -> Result<CommandOutput, AppError> {
	match args.runner_type {
		RunnerType::Kubernetes => {
			todo!("Kubernetes runner is not yet supported")
		}
		RunnerType::Docker => {}
	}

	let config_path = args
		.config
		.unwrap_or_else(|| crate::utils::runner_config_path(args.runner_type));

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
