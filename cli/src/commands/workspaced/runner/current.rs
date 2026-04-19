use clap::Args as ClapArgs;
use comfy_table::Table;
use common::prelude::{RunnerMode, RunnerSettings};
use docker::prelude::DockerSettings;
use models::api::workspace::runner::*;
use serde::Serialize;
use serde_json::Value;
use strum::IntoEnumIterator as _;

use crate::prelude::*;

/// JSON output shape for a self-hosted runner (mode = `selfHosted`).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SelfHostedOutput {
	/// Path to the config file on disk.
	config: String,
	/// Runner kind (`docker`, `kubernetes`).
	#[serde(rename = "type")]
	runner_type: String,
	/// Always `"selfHosted"` for this variant.
	mode: &'static str,
}

/// The arguments for the `runner current` command.
#[derive(Debug, Clone, ClapArgs)]
pub struct Args {}

/// Print information about the runner(s) configured on this host. Auto-detects
/// which runner type(s) are set up by looking for their config files — no
/// positional argument needed.
pub(super) async fn execute(_args: Args) -> Result<CommandOutput, AppError> {
	let configured = RunnerType::iter()
		.filter(|t| crate::utils::runner_config_path(*t).exists())
		.collect::<Vec<_>>();

	if configured.is_empty() {
		return Err(AppError::RunnerError(
			"No runner is configured on this host. Run `patr runner setup` to configure one."
				.to_string(),
		));
	}

	let multi = configured.len() > 1;
	let mut text_blocks = Vec::<String>::with_capacity(configured.len());
	let mut json_items = Vec::<Value>::with_capacity(configured.len());

	for runner_type in configured {
		let (table, json) = render_runner(runner_type).await?;
		let block = if multi {
			let header = match runner_type {
				RunnerType::Docker => "== docker ==",
				RunnerType::Kubernetes => "== kubernetes ==",
			};
			format!("{header}\n{table}")
		} else {
			table
		};
		text_blocks.push(block);
		json_items.push(json);
	}

	CommandOutput::builder()
		.text(text_blocks.join("\n\n"))
		.json(Value::Array(json_items))
		.build()
		.into_result()
}

/// Render a single configured runner — returns the text table plus the JSON
/// value that represents it.
async fn render_runner(runner_type: RunnerType) -> Result<(String, Value), AppError> {
	match runner_type {
		RunnerType::Kubernetes => {
			todo!("Kubernetes runner is not yet supported")
		}
		RunnerType::Docker => {}
	}

	let config_path = crate::utils::runner_config_path(runner_type);

	let config_str = std::fs::read_to_string(&config_path).map_err(|e| {
		AppError::RunnerError(format!(
			"Failed to read config file at {}: {e}",
			config_path.display()
		))
	})?;

	let config: RunnerSettings<DockerSettings> = serde_json::from_str(&config_str)
		.map_err(|e| AppError::RunnerError(format!("Failed to parse config: {e}")))?;

	let runner_type_str = match runner_type {
		RunnerType::Docker => "docker",
		RunnerType::Kubernetes => "kubernetes",
	};

	match config.mode {
		RunnerMode::SelfHosted { .. } => {
			let mut table = Table::new();
			table.add_row(["Config".to_string(), config_path.display().to_string()]);
			table.add_row(["Type".to_string(), runner_type_str.to_string()]);
			table.add_row(["Mode".to_string(), "Self-hosted".to_string()]);
			let json = SelfHostedOutput {
				config: config_path.display().to_string(),
				runner_type: runner_type_str.to_string(),
				mode: "selfHosted",
			}
			.to_json_value();
			Ok((table.to_string(), json))
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

			let json = GetRunnerInfoResponse { runner }.to_json_value();
			Ok((table.to_string(), json))
		}
	}
}
