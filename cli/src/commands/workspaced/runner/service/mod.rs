use std::process::Command;

use clap::Subcommand;

use crate::prelude::*;

/// Install the runner as a systemd service
mod install;
/// Show `systemctl status` for the installed runner service
mod status;
/// Stop, disable, and remove the installed runner service
mod uninstall;

/// The commands that can be executed on the runner's systemd service
#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum ServiceCommand {
	/// Install the runner as a systemd service
	Install(install::Args),
	/// Stop, disable, and remove the installed runner service
	#[command(alias = "remove", alias = "rm")]
	Uninstall(uninstall::Args),
	/// Show `systemctl status` for the installed runner service
	#[command(alias = "info")]
	Status(status::Args),
}

/// Dispatch a runner service subcommand
pub async fn execute(command: ServiceCommand) -> Result<CommandOutput, AppError> {
	match command {
		ServiceCommand::Install(args) => install::execute(args).await,
		ServiceCommand::Uninstall(args) => uninstall::execute(args).await,
		ServiceCommand::Status(args) => status::execute(args).await,
	}
}

/// Run `systemctl <args>` — directly when root, otherwise via `sudo`.
pub(super) fn run_systemctl(args: &[&str], is_root: bool) -> Result<(), AppError> {
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

/// Convert an `io::Error` from spawning `sudo` into a user-friendly
/// [`AppError::RunnerError`].
pub(super) fn sudo_spawn_error(e: std::io::Error, what: &str) -> AppError {
	if e.kind() == std::io::ErrorKind::NotFound {
		AppError::RunnerError(
			"sudo not found on PATH. Install sudo or run this command as root.".to_string(),
		)
	} else {
		AppError::RunnerError(format!("Failed to run {what}: {e}"))
	}
}
