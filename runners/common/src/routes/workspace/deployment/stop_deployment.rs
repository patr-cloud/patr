use http::StatusCode;
use models::{api::workspace::deployment::*, prelude::*};

use crate::prelude::*;

pub async fn stop_deployment(
	request: AppRequest<'_, StopDeploymentRequest>,
) -> Result<AppResponse<StopDeploymentRequest>, ErrorType> {
	let AppRequest {
		database,
		request:
			ProcessedApiRequest {
				path: StopDeploymentPath {
					workspace_id: _,
					deployment_id,
				},
				query: _,
				headers: _,
				body: _,
			},
	} = request;

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
