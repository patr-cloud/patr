use std::{
	path::{Path, PathBuf},
	process::Command,
};

use clap::Args as ClapArgs;

use crate::prelude::*;

/// The service file path for the systemd unit
const SERVICE_FILE_PATH: &str = "/etc/systemd/system/patr-runner.service";

/// The arguments for the `runner install-service` command.
#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
	/// The type of runner to install as a service
	#[arg(value_enum)]
	pub runner_type: RunnerType,
	/// Path to the config file (defaults to standard location for the runner
	/// type)
	#[arg(short = 'c', long = "config")]
	pub config: Option<PathBuf>,
}

pub async fn execute(args: Args) -> Result<CommandOutput, AppError> {
	// Check if systemd is available
	if !Path::new("/run/systemd/system").exists() {
		return Err(AppError::RunnerError(
			"systemd is not available on this system".to_string(),
		));
	}

	// The config path needs to be there
	let config_path = args
		.config
		.unwrap_or_else(|| crate::utils::runner_config_path(args.runner_type));

	if !config_path.exists() {
		return Err(AppError::RunnerError(format!(
			"Config file not found at {}. Run `patr runner setup` first.",
			config_path.display()
		)));
	}

	let config_path = config_path.canonicalize().map_err(|e| {
		AppError::RunnerError(format!(
			"Failed to resolve config path {}: {e}",
			config_path.display()
		))
	})?;

	// We need the binary path to set in the service file
	let patr_binary = std::env::current_exe()
		.and_then(|p| p.canonicalize())
		.map_err(|e| AppError::RunnerError(format!("Failed to determine patr binary path: {e}")))?;

	// Get current user/group (respecting sudo)
	let (username, groupname) = {
		let uid = if let Ok(sudo_uid) = std::env::var("SUDO_UID") {
			sudo_uid
				.parse::<u32>()
				.map_err(|e| AppError::RunnerError(format!("Invalid SUDO_UID: {e}")))?
		} else {
			uzers::get_current_uid()
		};

		let user = uzers::get_user_by_uid(uid).ok_or_else(|| {
			AppError::RunnerError(format!("Failed to resolve user for UID {uid}"))
		})?;

		let username = user
			.name()
			.to_str()
			.ok_or_else(|| AppError::RunnerError("Username is not valid UTF-8".to_string()))?
			.to_string();

		let group = uzers::get_group_by_gid(user.primary_group_id()).ok_or_else(|| {
			AppError::RunnerError(format!(
				"Failed to resolve group for GID {}",
				user.primary_group_id()
			))
		})?;

		let groupname = group
			.name()
			.to_str()
			.ok_or_else(|| AppError::RunnerError("Group name is not valid UTF-8".to_string()))?
			.to_string();

		(username, groupname)
	};

	// Generate unit file
	let runner_type_str = match args.runner_type {
		RunnerType::Docker => "docker",
		RunnerType::Kubernetes => "kubernetes",
	};

	// Write unit file
	if let Err(e) = std::fs::write(
		SERVICE_FILE_PATH,
		format!(
			include_str!("../../../../../assets/cli/systemd-service.template"),
			runner_type_str = runner_type_str,
			username = username,
			groupname = groupname,
			patr_binary = patr_binary.display(),
			config_path = config_path.display(),
		),
	) {
		if e.kind() == std::io::ErrorKind::PermissionDenied {
			return Err(AppError::RunnerError(format!(
				concat!(
					"Permission denied writing to {}. Re-run ",
					"with sudo: `sudo patr runner install-service {}`"
				),
				SERVICE_FILE_PATH, runner_type_str
			)));
		}
		return Err(AppError::RunnerError(format!(
			"Failed to write service file to {SERVICE_FILE_PATH}: {e}"
		)));
	}

	eprintln!("Service file written to {SERVICE_FILE_PATH}");

	// Interactive prompt to enable and start the service
	let enable_and_start = inquire::Confirm::new("Enable and start the service now?")
		.with_default(true)
		.prompt()
		.unwrap_or(false);

	if enable_and_start {
		Command::new("systemctl")
			.args(["daemon-reload"])
			.output()
			.map_err(|e| AppError::RunnerError(format!("Failed to run systemctl: {e}")))?;
		Command::new("systemctl")
			.args(["enable", "patr-runner.service"])
			.output()
			.map_err(|e| AppError::RunnerError(format!("Failed to run systemctl: {e}")))?;
		Command::new("systemctl")
			.args(["start", "patr-runner.service"])
			.output()
			.map_err(|e| AppError::RunnerError(format!("Failed to run systemctl: {e}")))?;

		CommandOutput::builder()
			.text("Patr runner service installed, enabled, and started.")
			.json(ApiSuccessResponseBody::empty().to_json_value())
			.build()
			.into_result()
	} else {
		let manual_commands = concat!(
			"To enable and start the service manually, run:\n",
			"sudo systemctl daemon-reload\n",
			"sudo systemctl enable patr-runner.service\n",
			"sudo systemctl start patr-runner.service"
		);

		CommandOutput::builder()
			.text(format!("Patr runner service installed.\n{manual_commands}"))
			.json(ApiSuccessResponseBody::empty().to_json_value())
			.build()
			.into_result()
	}
}
