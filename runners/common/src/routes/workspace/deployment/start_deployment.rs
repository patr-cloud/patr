use http::StatusCode;
use models::{api::workspace::deployment::*, prelude::*};

use crate::prelude::*;

/// The handler to start a deployment. This will start the deployment. In case
/// the deployment is already running, it will do nothing.
#[instrument(skip(database))]
pub async fn start_deployment(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: StartDeploymentPath {
					workspace_id: _,
					deployment_id,
				},
				query: StartDeploymentQuery { force_restart },
				headers:
					StartDeploymentRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: StartDeploymentRequestProcessed,
			},
		database,
		change_publisher: _,
		config: _,
	}: AppRequest<'_, StartDeploymentRequest>,
) -> Result<AppResponse<StartDeploymentRequest>, ErrorType> {
	trace!("Starting deployment: {}", deployment_id);

	query(
		r#"
		UPDATE
			deployment
		SET
			status = 'deploying'
		WHERE
			id = $1 AND (
				status != 'running' OR
				status != 'deploying'
			);
		"#,
	)
	.bind(deployment_id)
	.execute(&mut **database)
	.await?;

	AppResponse::builder()
		.body(StartDeploymentResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
