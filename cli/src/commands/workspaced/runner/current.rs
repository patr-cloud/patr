use clap::Args as ClapArgs;
use comfy_table::Table;
use common::prelude::RunnerSettings;
use docker::prelude::DockerSettings;
use models::api::workspace::runner::*;
use serde_json::Value;

use crate::prelude::*;

/// The arguments for the `runner current` command.
#[derive(Debug, Clone, ClapArgs)]
pub struct Args {}

/// Print information about the runner configured on this host. Auto-detects
/// whether one is set up by looking for its config file — no positional
/// argument needed.
pub(super) async fn execute(_args: Args) -> Result<CommandOutput, AppError> {
	if !crate::utils::runner_config_path().exists() {
		return Err(AppError::RunnerError(
			"No runner is configured on this host. Run `patr runner setup` to configure one."
				.to_string(),
		));
	}

	let (table, json) = render_runner().await?;

	CommandOutput::builder()
		.text(table)
		.json(Value::Array(vec![json]))
		.build()
		.into_result()
}

/// Render the configured runner — returns the text table plus the JSON value
/// that represents it.
async fn render_runner() -> Result<(String, Value), AppError> {
	let config_path = crate::utils::runner_config_path();

	let config_str = std::fs::read_to_string(&config_path).map_err(|e| {
		AppError::RunnerError(format!(
			"Failed to read config file at {}: {e}",
			config_path.display()
		))
	})?;

	let config: RunnerSettings<DockerSettings> = serde_json::from_str(&config_str)
		.map_err(|e| AppError::RunnerError(format!("Failed to parse config: {e}")))?;

	let runner_type_str = "docker";

	let RunnerSettings {
		workspace_id,
		runner_id,
		api_token,
		..
	} = config;

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
	table.add_row(["Workspace ID".to_string(), workspace_id.to_string()]);
	table.add_row(["Runner ID".to_string(), runner_id.to_string()]);
	table.add_row(["Runner Name".to_string(), runner.data.name.clone()]);
	table.add_row(["Status".to_string(), connected.to_string()]);

	let json = GetRunnerInfoResponse { runner }.to_json_value();
	Ok((table.to_string(), json))
}
