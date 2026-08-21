use std::{io::IsTerminal, path::Path, process::Command};

use clap::Args as ClapArgs;
use models::ApiSuccessResponseBody;

use crate::prelude::*;

/// Args for `patr uninstall`.
#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
	/// Skip the layer-1 confirmation prompt.
	#[arg(short = 'y', long = "yes")]
	pub yes: bool,
	/// Also tear down resources managed by Patr runners on this host
	/// (`docker swarm leave --force`). Implies `-y`.
	#[arg(long)]
	pub purge: bool,
}

/// Uninstall the Patr CLI from this host.
pub async fn execute(
	args: Args,
	_global_args: GlobalArgs,
	_state: AppState,
) -> Result<CommandOutput, AppError> {
	if cfg!(feature = "package-managed") {
		eprintln!("uninstall is disabled for this build of patr");
		eprintln!("this CLI is managed by your package manager — use it to uninstall");
		return CommandOutput::builder()
			.text(String::new())
			.json(ApiSuccessResponseBody::empty().to_json_value())
			.build()
			.into_result();
	}

	let current_exe = std::env::current_exe()
		.map_err(|e| AppError::RunnerError(format!("Failed to determine current binary: {e}")))?;

	let cli_config_path = crate::utils::config_local_dir();
	let runner_configs = Some(crate::utils::runner_config_path())
		.filter(|p| p.exists())
		.into_iter()
		.collect::<Vec<_>>();

	// Layer 1 confirm (unless -y or --purge).
	if !args.yes && !args.purge {
		if !std::io::stdin().is_terminal() {
			return Err(AppError::RunnerError(
				"Running in non-TTY mode. Pass -y to confirm uninstall.".to_string(),
			));
		}
		eprintln!("This will remove:");
		if cli_config_path.exists() {
			eprintln!("  - CLI config ({})", cli_config_path.display());
		}
		for p in &runner_configs {
			eprintln!("  - Runner config ({})", p.display());
		}
		if !runner_configs.is_empty() && Path::new("/run/systemd/system").exists() {
			eprintln!("  - Installed systemd services for the above runners, if any");
		}
		eprintln!("  - The patr binary at {}", current_exe.display());
		let confirm = inquire::Confirm::new("Continue?")
			.with_default(false)
			.prompt()
			.expect_tty("Failed to read uninstall confirmation");
		if !confirm {
			return Err(AppError::RunnerError("Aborted.".to_string()));
		}
	}

	// Layer 2: tear down runner-managed resources (docker swarm).
	let do_purge = if args.purge {
		true
	} else if !std::io::stdin().is_terminal() {
		false
	} else {
		eprintln!();
		eprintln!("Also tear down resources managed by Patr runners on this host?");
		eprintln!();
		eprintln!("This will run `docker swarm leave --force`.");
		eprintln!();
		eprintln!("Affected: services managed by this host's Docker swarm");
		eprintln!("(including Patr deployments running on this runner).");
		eprintln!(
			"Not affected: standalone containers running via `docker run` outside the swarm."
		);
		eprintln!();
		inquire::Confirm::new("Continue?")
			.with_default(false)
			.prompt()
			.expect_tty("Failed to read purge confirmation")
	};

	// Layer 2 runs first, while the runner configs + binary are still present.
	if do_purge {
		eprintln!("Running `docker swarm leave --force`...");
		match Command::new("docker")
			.args(["swarm", "leave", "--force"])
			.status()
		{
			Ok(status) if status.success() => {}
			Ok(status) => {
				eprintln!("docker swarm leave exited with {status} — continuing with uninstall.");
			}
			Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
				eprintln!("docker not found on PATH — skipping swarm cleanup.");
			}
			Err(e) => {
				eprintln!("Failed to run docker: {e} — continuing with uninstall.");
			}
		}
	}

	// Layer 1: remove the installed systemd service, if we found a runner
	// config on disk. Logic mirrors `runner service uninstall` but runs
	// non-interactively here.
	if Path::new("/run/systemd/system").exists() {
		let is_root = uzers::get_current_uid() == 0;
		if !is_root && !runner_configs.is_empty() {
			eprintln!(
				"This will use sudo to stop/remove installed systemd services. You may be prompted for your password."
			);
		}
		if !runner_configs.is_empty() {
			let service_name = "patr-docker-runner.service".to_string();
			let service_file_path = format!("/etc/systemd/system/{service_name}");

			// Lenient stop.
			let mut stop_cmd = if is_root {
				Command::new("systemctl")
			} else {
				let mut c = Command::new("sudo");
				c.arg("systemctl");
				c
			};
			stop_cmd.args(["stop", &service_name]);
			if let Err(e) = stop_cmd.status() {
				eprintln!("systemctl stop {service_name}: {e} (continuing)");
			}

			// Lenient disable.
			let mut disable_cmd = if is_root {
				Command::new("systemctl")
			} else {
				let mut c = Command::new("sudo");
				c.arg("systemctl");
				c
			};
			disable_cmd.args(["disable", &service_name]);
			if let Err(e) = disable_cmd.status() {
				eprintln!("systemctl disable {service_name}: {e} (continuing)");
			}

			let rm_ok = if is_root {
				std::fs::remove_file(&service_file_path)
					.or_else(|e| {
						if e.kind() == std::io::ErrorKind::NotFound {
							Ok(())
						} else {
							Err(e)
						}
					})
					.map_err(|e| format!("Failed to remove {service_file_path}: {e}"))
			} else {
				Command::new("sudo")
					.args(["rm", "-f", &service_file_path])
					.status()
					.map_err(|e| format!("Failed to run sudo rm: {e}"))
					.and_then(|s| {
						if s.success() {
							Ok(())
						} else {
							Err(format!("sudo rm exited with {s}"))
						}
					})
			};
			if let Err(e) = rm_ok {
				eprintln!("Warning: {e}");
			}
		}

		// One daemon-reload after all services removed.
		if !runner_configs.is_empty() {
			let _ = if is_root {
				Command::new("systemctl")
			} else {
				let mut c = Command::new("sudo");
				c.arg("systemctl");
				c
			}
			.arg("daemon-reload")
			.status();
		}
	}

	// Remove runner config files.
	for config_path in &runner_configs {
		if let Err(e) = std::fs::remove_file(config_path) &&
			e.kind() != std::io::ErrorKind::NotFound
		{
			eprintln!("Warning: failed to remove {}: {e}", config_path.display());
		}
	}

	// Remove the CLI state file.
	if cli_config_path.exists() &&
		let Err(e) = std::fs::remove_file(&cli_config_path)
	{
		eprintln!(
			"Warning: failed to remove {}: {e}",
			cli_config_path.display()
		);
	}

	// Remove the binary last (it's the one executing this code). Try
	// self_delete first — falls back to `sudo rm` when the path is
	// root-owned (typical for /usr/local/bin).
	match self_replace::self_delete() {
		Ok(()) => {}
		Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
			eprintln!(
				"Removing {} (may prompt for your password).",
				current_exe.display()
			);
			let status = Command::new("sudo")
				.args(["rm", "-f"])
				.arg(&current_exe)
				.status()
				.map_err(|e| match e.kind() {
					std::io::ErrorKind::NotFound => AppError::RunnerError(
						"sudo not found on PATH. Remove the binary manually.".to_string(),
					),
					_ => AppError::RunnerError(format!("Failed to run sudo rm: {e}")),
				})?;
			if !status.success() {
				return Err(AppError::RunnerError(format!(
					"sudo rm {} exited with {status}",
					current_exe.display()
				)));
			}
		}
		Err(err) => {
			return Err(AppError::RunnerError(format!(
				"Failed to delete running binary: {err}"
			)));
		}
	}

	CommandOutput::builder()
		.text("Patr CLI uninstalled.".to_string())
		.json(ApiSuccessResponseBody::empty().to_json_value())
		.build()
		.into_result()
}
