use http::StatusCode;
use models::{api::workspace::deployment::*, prelude::*};

use crate::prelude::*;

/// The handler to start a deployment. This will start the deployment. In case
/// the deployment is already running, it will do nothing.
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
		runner_changes_sender: _,
		config: _,
	}: AppRequest<'_, StartDeploymentRequest>,
) -> Result<AppResponse<StartDeploymentRequest>, ErrorType> {
	trace!("Starting deployment: {}", deployment_id);

	let _ = query(
		r#"
		SELECT 
			registry,
			image_name,
			image_tag
		FROM
			deployment
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
	)
	.bind(deployment_id)
	.fetch_optional(&mut **database)
	.await?
	.map(|row| {
		let registry = row.try_get::<String, _>("registry")?;
		let image_tag = row.try_get::<String, _>("image_tag")?;
		let image_name = row.try_get::<String, _>("image_name")?;

		Ok::<_, ErrorType>((registry, image_tag, image_name))
	})
	.transpose()?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	query(
		r#"
		UPDATE
			deployment
		SET
			status = 'deploying'
		WHERE
			id = $1
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
