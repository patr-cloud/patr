use std::{io, pin::pin, process::Stdio};

use fslock::LockFile;
use futures::{TryFutureExt, future};
use tokio::{
	fs,
	process::Command,
	time::{self, Duration},
};

use crate::prelude::*;

impl<E> super::Runner<E>
where
	E: RunnerExecutor + Send + Sync + 'static,
{
	/// Run nginx. This function will start nginx and listen for incoming
	/// connections. It will return a result with the error if nginx fails
	/// to start. Nginx will run until the exit signal is received.
	#[instrument(skip(self))]
	pub(super) async fn run_nginx(&self) -> Result<!, RunnerError> {
		fs::create_dir_all("./data/nginx")
			.await
			.map_err(RunnerError::NginxSetupError)?;

		fs::write(
			constants::NGINX_CONFIG_PATH,
			include_str!(concat!(
				env!("CARGO_MANIFEST_DIR"),
				"/../../assets/runner/nginx.conf"
			)),
		)
		.await
		.map_err(RunnerError::NginxSetupError)?;

		loop {
			let mut receiver = self.state.nginx_reload_notifier.notified();

			let mut lock_file = LockFile::open(constants::NGINX_LOCK_FILE_PATH)
				.map_err(RunnerError::NginxSetupError)?;

			if !lock_file.owns_lock() {
				// remove the nginx socket file if it exists based on a lockfile
				let locked = lock_file.try_lock().map_err(RunnerError::NginxSetupError)?;
				if !locked {
					error!("Failed to acquire lock for nginx. Another instance might be running");
					time::sleep(Duration::from_secs(5))
						.with_cancel_check()
						.await?;
					continue;
				}

				if fs::try_exists(constants::NGINX_SOCKET_PATH)
					.await
					.map_err(RunnerError::NginxSetupError)?
				{
					warn!("Removing existing nginx socket file");
					fs::remove_file(constants::NGINX_SOCKET_PATH)
						.await
						.map_err(RunnerError::NginxSetupError)
						.inspect_err(|err| {
							error!("Failed to remove nginx socket file: {:?}", err);
						})?;
				}
			}

			let Ok(mut child) = Command::new("nginx")
				.arg("-g")
				.arg("daemon off;")
				.arg("-p")
				.arg(".")
				.arg("-e")
				.arg("./data/nginx/error.log")
				.arg("-c")
				.arg(constants::NGINX_CONFIG_PATH)
				.env(
					"PATH",
					format!(
						"{}:{}",
						self.temp_dir.path().display(),
						std::env::var("PATH").unwrap_or_default()
					),
				)
				.stdin(Stdio::piped())
				.stdout(Stdio::piped())
				.stderr(Stdio::piped())
				.kill_on_drop(true)
				.spawn()
				.inspect_err(|err| {
					error!("Failed to start nginx: {:?}", err);
				})
			else {
				// Retry after 5 seconds, but break if the exit signal is received
				time::sleep(Duration::from_secs(5))
					.with_cancel_check()
					.await?;
				continue;
			};

			let status = loop {
				let Some(status) = future::select(
					pin!(child.wait().map_err(RunnerError::NginxExecError)),
					pin!(receiver),
				)
				.with_cancel_check()
				.await?
				.into_left() else {
					info!("Reloading nginx configuration");
					if let Err(err) = self.reload_nginx().await {
						error!("Failed to reload nginx: {:?}", err);
					} else {
						info!("Nginx configuration reloaded successfully");
					}
					receiver = self.state.nginx_reload_notifier.notified();
					continue;
				};
				break status?;
			};

			if status.success() {
				warn!("Nginx exited successfully");
				future::ready(()).with_cancel_check().await?;
				warn!("This should not happen. Restarting nginx");
			} else {
				error!("Nginx exited with status: {}", status);
			}
		}
	}

	/// Reload nginx configuration. This function will send a reload signal to
	/// nginx, causing it to reload its configuration. It will return a result
	/// with the error if nginx fails to reload. This function is useful when
	/// the nginx configuration is changed and needs to be reloaded without
	/// restarting the nginx process.
	#[instrument(skip(self))]
	pub(super) async fn reload_nginx(&self) -> Result<(), RunnerError> {
		let output = Command::new("nginx")
			.arg("-s")
			.arg("reload")
			.arg("-p")
			.arg(".")
			.arg("-e")
			.arg("./data/nginx/error.log")
			.arg("-c")
			.arg(constants::NGINX_CONFIG_PATH)
			.env(
				"PATH",
				format!(
					"{}:{}",
					self.temp_dir.path().display(),
					std::env::var("PATH").unwrap_or_default()
				),
			)
			.stdin(Stdio::piped())
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.output()
			.await
			.inspect_err(|err| {
				error!("Failed to reload nginx: {:?}", err);
			})
			.map_err(RunnerError::NginxExecError)?;

		if !output.status.success() {
			let stderr = String::from_utf8_lossy(&output.stderr);
			error!("Nginx reload failed: {}", stderr);
			return Err(RunnerError::NginxExecError(io::Error::other(
				"Failed to reload nginx",
			)));
		}

		Ok(())
	}
}
