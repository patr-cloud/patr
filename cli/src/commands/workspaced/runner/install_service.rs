use std::{
	io::Write,
	path::{Path, PathBuf},
	process::{Command, Stdio},
};

use clap::Args as ClapArgs;

use crate::prelude::*;

/// The arguments for the `runner install-service` command.
///
/// Run this command without sudo. Patr resolves your config and renders the
/// systemd unit as your user, then uses `sudo` to write the unit file and run
/// `systemctl`. You may be prompted for your password.
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
	if !Path::new("/run/systemd/system").exists() {
		return Err(AppError::RunnerError(
			"systemd is not available on this system".to_string(),
		));
	}

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

	let patr_binary = std::env::current_exe()
		.and_then(|p| p.canonicalize())
		.map_err(|e| AppError::RunnerError(format!("Failed to determine patr binary path: {e}")))?;

	let uid = uzers::get_current_uid();
	let user = uzers::get_user_by_uid(uid)
		.ok_or_else(|| AppError::RunnerError(format!("Failed to resolve user for UID {uid}")))?;

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

	let runner_type_str = match args.runner_type {
		RunnerType::Docker => "docker",
		RunnerType::Kubernetes => "kubernetes",
	};

	let service_name = format!("patr-{runner_type_str}-runner.service");
	let service_file_path = format!("/etc/systemd/system/{service_name}");

	let unit_file = format!(
		include_str!("../../../../../assets/cli/systemd-service.template"),
		runner_type_str = runner_type_str,
		username = username,
		groupname = groupname,
		patr_binary = patr_binary.display(),
		config_path = config_path.display(),
	);

	let enable_and_start = inquire::Confirm::new("Enable and start the service now?")
		.with_default(true)
		.prompt()
		.unwrap_or(false);

	let is_root = uid == 0;

	if !is_root {
		eprintln!(
			"This will use sudo to write {service_file_path} and run systemctl. You may be prompted for your password."
		);
	}

	if is_root {
		std::fs::write(&service_file_path, &unit_file).map_err(|e| {
			AppError::RunnerError(format!(
				"Failed to write service file to {service_file_path}: {e}"
			))
		})?;
	} else {
		let mut child = Command::new("sudo")
			.args(["tee", &service_file_path])
			.stdin(Stdio::piped())
			.stdout(Stdio::null())
			.stderr(Stdio::inherit())
			.spawn()
			.map_err(|e| sudo_spawn_error(e, "sudo tee"))?;

		child
			.stdin
			.as_mut()
			.expect("stdin was piped")
			.write_all(unit_file.as_bytes())
			.map_err(|e| {
				AppError::RunnerError(format!("Failed to pipe unit file to sudo tee: {e}"))
			})?;

		let status = child
			.wait()
			.map_err(|e| AppError::RunnerError(format!("Failed to wait for sudo tee: {e}")))?;

		if !status.success() {
			return Err(AppError::RunnerError(format!(
				"Failed to write {service_file_path} via sudo (exit status {status})"
			)));
		}
	}

	eprintln!("Service file written to {service_file_path}");

	if enable_and_start {
		run_systemctl(&["daemon-reload"], is_root)?;
		run_systemctl(&["enable", &service_name], is_root)?;
		run_systemctl(&["start", &service_name], is_root)?;

		CommandOutput::builder()
			.text(format!(
				"Patr runner service `{service_name}` installed, enabled, and started."
			))
			.json(ApiSuccessResponseBody::empty().to_json_value())
			.build()
			.into_result()
	} else {
		let manual_commands = format!(
			"To enable and start the service manually, run:\n\
			 sudo systemctl daemon-reload\n\
			 sudo systemctl enable {service_name}\n\
			 sudo systemctl start {service_name}"
		);

		CommandOutput::builder()
			.text(format!(
				"Patr runner service `{service_name}` installed.\n{manual_commands}"
			))
			.json(ApiSuccessResponseBody::empty().to_json_value())
			.build()
			.into_result()
	}
}

/// Run `systemctl <args>` — directly when root, otherwise via `sudo`.
fn run_systemctl(args: &[&str], is_root: bool) -> Result<(), AppError> {
	let status = if is_root {
		Command::new("systemctl")
			.args(args)
			.status()
			.map_err(|e| AppError::RunnerError(format!("Failed to run systemctl: {e}")))?
	} else {
		let mut full_args = Vec::<&str>::with_capacity(args.len() + 1);
		full_args.push("systemctl");
		full_args.extend_from_slice(args);
		Command::new("sudo")
			.args(&full_args)
			.status()
			.map_err(|e| sudo_spawn_error(e, "sudo systemctl"))?
	};

	if !status.success() {
		return Err(AppError::RunnerError(format!(
			"systemctl {} failed (exit status {status})",
			args.join(" ")
		)));
	}

	Ok(())
}

fn sudo_spawn_error(e: std::io::Error, what: &str) -> AppError {
	if e.kind() == std::io::ErrorKind::NotFound {
		AppError::RunnerError(
			"sudo not found on PATH. Install sudo or run this command as root.".to_string(),
		)
	} else {
		AppError::RunnerError(format!("Failed to run {what}: {e}"))
	}
}
