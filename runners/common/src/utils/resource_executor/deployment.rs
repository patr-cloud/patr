use tokio_util::sync::CancellationToken;

use crate::prelude::*;

/// The resource executor task that will be used to specifically manage the
/// deployments of the runner.
pub(super) async fn handle_deployment<E>(
	resource_id: Uuid,
	executor: E,
	state: AppState<E>,
	cancellation_token: CancellationToken,
) -> Result<(), RunnerError>
where
	E: RunnerExecutor,
{
	// Keep checking for the status of the deployment and
	// update the database
	loop {
		let Ok(status) = executor
			.get_deployment_status(resource_id)
			.with_cancel_check_of(&cancellation_token)
			.await?
		else {
			continue;
		};

		// Update the status of the deployment in the database
		query(
			r#"
			UPDATE
				deployment
			SET
				status = $1
			WHERE
				id = $2;
			"#,
		)
		.bind(status.to_string())
		.bind(resource_id)
		.execute(&state.database)
		.await?;

		if cancellation_token.is_cancelled() {
			return Err(RunnerError::ExitSignalReceived);
		}
	}
}
