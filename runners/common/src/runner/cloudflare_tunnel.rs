use std::process::Stdio;

use futures::future;
use models::api::workspace::runner::*;
use tokio::{
	process::Command,
	time::{self, Duration},
};

use crate::{prelude::*, utils::client::make_request};

impl<E> super::Runner<E>
where
	E: RunnerExecutor + Send + Sync + 'static,
{
	/// Run the cloudflare tunnel. This function will start the cloudflare
	/// tunnel and listen for incoming connections. It will return a result
	/// with the error if the tunnel fails to start. The tunnel will run until
	/// the exit signal is received.
	#[instrument(skip(self))]
	pub(super) async fn run_cloudflare_tunnel(&self) -> Result<!, RunnerError> {
		let RunnerMode::Managed {
			workspace_id,
			runner_id,
			api_token,
			user_agent,
		} = self.state.config.mode.clone()
		else {
			// If the runner is running in self-hosted mode, return early. The run function
			// uses a join of all the futures so early return here will not stop the runner
			// from running
			debug!("Runner is running in self-hosted mode. Skipping cloudflare tunnel");
			return Err(RunnerError::Unsupported);
		};

		let runner_exposure_type = E::runner_exposure_type(&self.state.config);

		if !runner_exposure_type.requires_tunnel() {
			return Err(RunnerError::Unsupported);
		}

		info!("Running cloudflare tunnel to expose the runner");
		loop {
			let tunnel_token = make_request(
				ApiRequest::<GetIngressTokenForRunnerRequest>::builder()
					.path(GetIngressTokenForRunnerPath {
						workspace_id,
						runner_id,
					})
					.headers(GetIngressTokenForRunnerRequestHeaders {
						authorization: api_token.clone(),
						user_agent: user_agent.clone(),
					})
					.build(),
			)
			.with_cancel_check()
			.await?;

			let Ok(tunnel_token) = tunnel_token
				.inspect_err(|err| {
					error!("Failed to connect to the server: {:?}", err);
					error!("Retrying in 5 second");
				})
				.map_err(|err| err.body)
			else {
				// Retry after 5 seconds, but break if the exit signal is received
				time::sleep(Duration::from_secs(5))
					.with_cancel_check()
					.await?;
				continue;
			};

			let Ok(mut child) = Command::new("cloudflared")
				.arg("tunnel")
				.arg("--logfile")
				.arg("./data/cloudflared.log")
				.arg("run")
				.arg("--token")
				.arg(tunnel_token.body.token)
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
					error!("Failed to start cloudflare tunnel: {:?}", err);
					error!("Retrying in 5 second");
				})
			else {
				// Retry after 5 seconds, but break if the exit signal is received
				time::sleep(Duration::from_secs(5))
					.with_cancel_check()
					.await?;
				continue;
			};

			let status = match child.wait().with_cancel_check().await {
				Ok(status) => status,
				Err(err) => {
					// Exit signal received. Kill the child process and exit
					child
						.kill()
						.await
						.map_err(RunnerError::CloudflareTunnelExecError)?;
					child
						.wait()
						.await
						.map_err(RunnerError::CloudflareTunnelExecError)?;
					return Err(err);
				}
			};

			let Ok(status) = status.inspect_err(|err| {
				error!("Error waiting for cloudflared process: {}", err);
				error!("Retrying in 5 second");
			}) else {
				// Retry after 5 seconds, but break if the exit signal is received
				if let Err(RunnerError::ExitSignalReceived) = time::sleep(Duration::from_secs(5))
					.with_cancel_check()
					.await
				{
					// Exit signal received. Kill the child process and exit
					child
						.kill()
						.await
						.map_err(RunnerError::CloudflareTunnelExecError)?;
					child
						.wait()
						.await
						.map_err(RunnerError::CloudflareTunnelExecError)?;
					return Err(RunnerError::ExitSignalReceived);
				}
				continue;
			};

			if status.success() {
				warn!("Cloudflare tunnel exited successfully");
				future::ready(()).with_cancel_check().await?;
				warn!("This should not happen. Restarting tunnel");
			} else {
				error!("Cloudflare tunnel exited with status: {}", status);
				error!("Retrying in 1 second");
				// Retry after a second, but break if the exit signal is received
				time::sleep(Duration::from_secs(1))
					.with_cancel_check()
					.await?;
			}
		}
	}
}
