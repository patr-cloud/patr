use std::time::Duration;

use http::StatusCode;
use models::{api::workspace::deployment::*, prelude::*};

use crate::{actors::runner_supervisor::RunnerSupervisorMessage, prelude::*};

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
		config: _,
		supervisor_ref,
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

	supervisor_ref.send_after(Duration::from_millis(50), move || {
		RunnerSupervisorMessage::UpsertResource {
			resource_id: deployment_id,
			resource_type: ResourceType::Deployment,
		}
	});

	AppResponse::builder()
		.body(StopDeploymentResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
