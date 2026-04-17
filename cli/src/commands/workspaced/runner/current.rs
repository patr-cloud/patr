use std::path::PathBuf;

use clap::Args as ClapArgs;
use comfy_table::Table;
use common::prelude::{RunnerMode, RunnerSettings};
use docker::prelude::DockerSettings;
use models::api::workspace::runner::*;

use crate::prelude::*;

/// The arguments for the `runner current` command.
#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
	/// The type of runner configured on this host
	#[arg(value_enum)]
	pub runner_type: RunnerType,
	/// Path to the config file (defaults to standard location for the runner
	/// type)
	#[arg(short = 'c', long = "config")]
	pub config: Option<PathBuf>,
}

/// Print information about the runner configured on this host.
pub(super) async fn execute(args: Args) -> Result<CommandOutput, AppError> {
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

	let runner_type_str = match args.runner_type {
		RunnerType::Docker => "docker",
		RunnerType::Kubernetes => "kubernetes",
	};

	match config.mode {
		RunnerMode::SelfHosted { .. } => {
			let mut table = Table::new();
			table.add_row(["Config".to_string(), config_path.display().to_string()]);
			table.add_row(["Type".to_string(), runner_type_str.to_string()]);
			table.add_row(["Mode".to_string(), "Self-hosted".to_string()]);
			CommandOutput::builder()
				.text(table.to_string())
				.json(
					serde_json::json!({
						"config": config_path.display().to_string(),
						"type": runner_type_str,
						"mode": "selfHosted",
					}),
				)
				.build()
				.into_result()
		}
		RunnerMode::Managed {
			workspace_id,
			runner_id,
			api_token,
			..
		} => {
			let runner = make_request(
				ApiRequest::<GetRunnerInfoRequest>::builder()
					.path(GetRunnerInfoPath {
						workspace_id,
						runner_id,
					})
					.headers(GetRunnerInfoRequestHeaders {
						authorization: api_token,
						user_agent: constants::USER_AGENT,
					})
					.build(),
			)
			.await?
			.body
			.runner;

			let connected = if runner.data.connected {
				"✅ Connected"
			} else {
				"❌ Disconnected"
			};

			let mut table = Table::new();
			table.add_row(["Config".to_string(), config_path.display().to_string()]);
			table.add_row(["Type".to_string(), runner_type_str.to_string()]);
			table.add_row(["Mode".to_string(), "Managed".to_string()]);
			table.add_row(["Workspace ID".to_string(), workspace_id.to_string()]);
			table.add_row(["Runner ID".to_string(), runner_id.to_string()]);
			table.add_row(["Runner Name".to_string(), runner.data.name.clone()]);
			table.add_row(["Status".to_string(), connected.to_string()]);

			CommandOutput::builder()
				.text(table.to_string())
				.json(GetRunnerInfoResponse { runner }.to_json_value())
				.build()
				.into_result()
		}
	}
}
