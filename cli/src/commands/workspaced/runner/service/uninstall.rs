use std::{io::IsTerminal, path::Path, process::Command};

use clap::Args as ClapArgs;

use super::{run_systemctl, sudo_spawn_error};
use crate::prelude::*;

/// The arguments for the `runner service uninstall` command.
#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
	/// The type of runner whose service to remove
	#[arg(value_enum)]
	pub runner_type: RunnerType,
	/// Skip the confirmation prompt
	#[arg(short = 'y', long = "yes")]
	pub yes: bool,
}

/// Stop, disable, and remove the installed runner systemd service.
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
	let service_file_path = format!("/etc/systemd/system/{service_name}");

	let is_root = uzers::get_current_uid() == 0;

	if !args.yes {
		if !std::io::stdin().is_terminal() {
			return Err(AppError::RunnerError(
				"Running in non-TTY mode. Pass `-y` to confirm.".to_string(),
			));
		}
		let confirmed =
			inquire::Confirm::new(&format!("Stop, disable, and remove {service_name}?"))
				.with_default(false)
				.prompt()
				.expect_tty("Failed to read uninstall confirmation");
		if !confirmed {
			return Err(AppError::RunnerError("Aborted.".to_string()));
		}
	}

	if !is_root {
		eprintln!(
			"This will use sudo to run systemctl and remove {service_file_path}. You may be prompted for your password."
		);
	}

	if let Err(err) = run_systemctl(&["stop", &service_name], is_root) {
		eprintln!("systemctl stop: {err} (continuing)");
	}
	if let Err(err) = run_systemctl(&["disable", &service_name], is_root) {
		eprintln!("systemctl disable: {err} (continuing)");
	}

	if is_root {
		match std::fs::remove_file(&service_file_path) {
			Ok(()) => {}
			Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
			Err(e) => {
				return Err(AppError::RunnerError(format!(
					"Failed to remove {service_file_path}: {e}"
				)));
			}
		}
	} else {
		let status = Command::new("sudo")
			.args(["rm", "-f", &service_file_path])
			.status()
			.map_err(|e| sudo_spawn_error(e, "sudo rm"))?;
		if !status.success() {
			return Err(AppError::RunnerError(format!(
				"Failed to remove {service_file_path} via sudo (exit status {status})"
			)));
		}
	}

	run_systemctl(&["daemon-reload"], is_root)?;

	CommandOutput::builder()
		.text(format!("Patr runner service `{service_name}` uninstalled."))
		.json(ApiSuccessResponseBody::empty().to_json_value())
		.build()
		.into_result()
}
