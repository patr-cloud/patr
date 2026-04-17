use std::{
	io::Write as _,
	process::{Command, Stdio},
};

use models::ApiSuccessResponseBody;

use crate::prelude::*;

/// Log the local docker CLI into registry.patr.cloud using the current Patr
/// session's API token. Username is the literal string `patr` — the server's
/// docker-login handler enforces this.
pub(super) async fn execute(
	_global_args: GlobalArgs,
	state: AppState,
) -> Result<CommandOutput, AppError> {
	let AppState::LoggedIn { token, .. } = state else {
		return Err(AppError::NotLoggedIn);
	};

	let mut child = Command::new("docker")
		.args([
			"login",
			"registry.patr.cloud",
			"-u",
			"patr",
			"--password-stdin",
		])
		.stdin(Stdio::piped())
		.stdout(Stdio::null())
		.stderr(Stdio::inherit())
		.spawn()
		.map_err(|e| match e.kind() {
			std::io::ErrorKind::NotFound => AppError::RunnerError(
				"docker not found on PATH. Install Docker first.".to_string(),
			),
			_ => AppError::RunnerError(format!("Failed to run `docker login`: {e}")),
		})?;

	child
		.stdin
		.as_mut()
		.expect("stdin was piped")
		.write_all(token.0.token().as_bytes())
		.map_err(|e| AppError::RunnerError(format!("Failed to pipe token to docker login: {e}")))?;

	let status = child
		.wait()
		.map_err(|e| AppError::RunnerError(format!("Failed to wait for docker login: {e}")))?;

	if !status.success() {
		return Err(AppError::RunnerError(format!(
			"`docker login` failed (exit status {status})"
		)));
	}

	CommandOutput::builder()
		.text("Logged in to registry.patr.cloud.")
		.json(ApiSuccessResponseBody::empty().to_json_value())
		.build()
		.into_result()
}
