use axum::http::StatusCode;
use models::{api::workspace::deployment::deploy_history::*, rbac::ResourceType};

use crate::{
	prelude::*,
	redis_publisher::{self, DirtyResource},
};
/// Revert a deployment to a previous deployment history, using the image
/// digest.
pub async fn revert_deployment(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					RevertDeploymentPath {
						workspace_id,
						deployment_id,
						image_digest,
					},
				query: (),
				headers:
					RevertDeploymentRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: RevertDeploymentRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		actor_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, RevertDeploymentRequest>,
) -> Result<AppResponse<RevertDeploymentRequest>, ErrorType> {
	info!(
		"Reverting deployment `{}` to image: {}",
		deployment_id, image_digest
	);

	// Revert the deployment to the specified image digest if the deployment exists
	let exists = query!(
		r#"
		SELECT
			*
		FROM
			deployment_deploy_history
		WHERE
			deployment_id = $1 AND
			image_digest = $2;
		"#,
		deployment_id,
		image_digest
	)
	.fetch_optional(&mut **database)
	.await?
	.is_some();

	if !exists {
		info!(
			"Deployment history with deployment ID `{}` and image digest `{}` does not exist",
			deployment_id, image_digest
		);
		return Err(ErrorType::ResourceDoesNotExist);
	}

	let runner = query!(
		r#"
		UPDATE
			deployment
		SET
			current_live_digest = $1
		WHERE
			id = $2
		RETURNING
			runner;
		"#,
		image_digest,
		deployment_id
	)
	.fetch_one(&mut **database)
	.await?
	.runner;

	redis_publisher::mark_dirty(
		database,
		DirtyResource {
			workspace_id,
			runner_id: runner,
			resource_type: ResourceType::Deployment,
			resource_id: deployment_id,
		},
	)
	.await?;

	AppResponse::builder()
		.body(RevertDeploymentResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
