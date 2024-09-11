use http::StatusCode;
use models::{api::workspace::deployment::*, prelude::*};

use crate::prelude::*;

pub async fn start_deployment(
	request: AppRequest<'_, StartDeploymentRequest>,
) -> Result<AppResponse<StartDeploymentRequest>, ErrorType> {
	let AppRequest {
		database,
		request:
			ProcessedApiRequest {
				path: StartDeploymentPath {
					workspace_id: _,
					deployment_id,
				},
				query: _,
				headers: _,
				body: _,
			},
	} = request;

	let (_registry, _image_name, _image_tag) = query(
		r#"
		SELECT 
			registry,
			image_name,
			image_tag
		FROM
			deployment
		WHERE
			id = $1,
			deleted IS NULL;
		"#,
	)
	.bind(deployment_id)
	.fetch_optional(&mut **database)
	.await?
	.map(|row| -> Result<(String, String, String), ErrorType> {
		let registry = row
			.try_get::<String, &str>("registry")
			.map_err(|_| ErrorType::server_error("corrupted deployment, cannot find registry"))?;
		let image_tag = row
			.try_get::<String, &str>("image_tag")
			.map_err(|_| ErrorType::server_error("corrupted deployment, cannot find image tag"))?;
		let image_name = row
			.try_get::<String, &str>("image_name")
			.map_err(|_| ErrorType::server_error("corrupted deployment, cannot find image name"))?;

		Ok((registry, image_tag, image_name))
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
