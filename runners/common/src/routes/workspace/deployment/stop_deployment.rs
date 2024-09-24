use http::StatusCode;
use models::{api::workspace::deployment::*, prelude::*};

use crate::prelude::*;

/// The handler to stop a deployment. This will stop the deployment. In case the
/// deployment is already stopped, it will do nothing.
pub async fn stop_deployment(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: StopDeploymentPath {
					workspace_id: _,
					deployment_id,
				},
				query: (),
				headers:
					StopDeploymentRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: StopDeploymentRequestProcessed,
			},
		database,
		runner_changes_sender: _,
		config: _,
	}: AppRequest<'_, StopDeploymentRequest>,
) -> Result<AppResponse<StopDeploymentRequest>, ErrorType> {
	trace!("Stopping deployment: {}", deployment_id);

	query(
		r#"
		UPDATE
			deployment
		SET
			status = 'stopped'
		WHERE
			id = $1
		"#,
	)
	.bind(deployment_id)
	.execute(&mut **database)
	.await?;

	AppResponse::builder()
		.body(StopDeploymentResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
