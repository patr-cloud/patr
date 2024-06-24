use axum::http::StatusCode;
use models::api::workspace::deployment::deploy_history::*;

use crate::prelude::*;

/// Delete a deployment's particular history of deploys, using the image digest.
pub async fn delete_deploy_history(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					DeleteDeploymentDeployHistoryPath {
						workspace_id: _,
						deployment_id,
						image_digest,
					},
				query: (),
				headers:
					DeleteDeploymentDeployHistoryRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: DeleteDeploymentDeployHistoryRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, DeleteDeploymentDeployHistoryRequest>,
) -> Result<AppResponse<DeleteDeploymentDeployHistoryRequest>, ErrorType> {
	info!(
		"Deleting deployment `{}`'s deploy history: {}",
		deployment_id, image_digest
	);

	// Delete the deployment history if the deployment exists
	let rows_affected = query!(
		r#"
		DELETE FROM
			deployment_deploy_history
		WHERE
			deployment_id = $1 AND
			image_digest = $2;
		"#,
		deployment_id as _,
		image_digest
	)
	.execute(&mut **database)
	.await?
	.rows_affected();

	if rows_affected == 0 {
		return Err(ErrorType::ResourceDoesNotExist);
	}

	AppResponse::builder()
		.body(DeleteDeploymentDeployHistoryResponse)
		.headers(())
		.status_code(StatusCode::RESET_CONTENT)
		.build()
		.into_result()
}
