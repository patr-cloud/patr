use std::{
	path::Path,
	process::{Command, Stdio},
};

use clap::Args as ClapArgs;
use serde::Serialize;

use crate::prelude::*;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusOutput {
	service: String,
	exit_status: Option<i32>,
}

/// The arguments for the `runner service status` command.
#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
	/// The type of runner whose service status to show
	#[arg(value_enum)]
	pub runner_type: RunnerType,
}

/// Show `systemctl status` for the installed runner service.
pub async fn execute(args: Args) -> Result<CommandOutput, AppError> {
	if !Path::new("/run/systemd/system").exists() {
		return Err(AppError::RunnerError(
			"systemd is not available on this system".to_string(),
		));
	}

	let runner_type_str = match args.runner_type {
		RunnerType::Docker => "docker",
		RunnerType::Kubernetes => "kubernetes",
	};
	let service_name = format!("patr-{runner_type_str}-runner.service");

	let status = Command::new("systemctl")
		.args(["status", &service_name])
		.stdout(Stdio::inherit())
		.stderr(Stdio::inherit())
		.status()
		.map_err(|e| AppError::RunnerError(format!("Failed to run systemctl: {e}")))?;

	CommandOutput::builder()
		.text(String::new())
		.json(
			StatusOutput {
				service: service_name,
				exit_status: status.code(),
			}
			.to_json_value(),
		)
		.build()
		.into_result()
}
