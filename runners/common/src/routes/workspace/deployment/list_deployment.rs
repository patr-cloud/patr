use http::StatusCode;
use models::api::workspace::deployment::*;
use sqlx::types::Uuid;

use crate::{
	app::{AppRequest, ProcessedApiRequest},
	prelude::*,
};

pub async fn list_deployment(
	request: AppRequest<'_, ListDeploymentRequest>,
) -> Result<AppResponse<ListDeploymentRequest>, ErrorType> {
	let AppRequest {
		database,
		request: ProcessedApiRequest {
			path: _,
			query: _,
			headers: _,
			body: _,
		},
	} = request;

	let deployments = query(
		r#"
		 SELECT
			id,
			name,
			registry,
			image_name,
			image_tag,
			machine_type,
			current_live_digest,
		FROM
			deployment
		"#,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		let deployment_id = row.try_get::<String, &str>("id")?;
		let name = row.try_get::<String, &str>("name")?;
		let deployment_id =
			Uuid::parse_str(&deployment_id).expect("deployment id to be valid uuid");

		let image_tag = row.try_get::<String, &str>("image_tag")?;
		let status = row.try_get::<DeploymentStatus, &str>("status")?;
		let registry = row.try_get::<String, &str>("registry")?;
		let image_name = row.try_get::<String, &str>("image_name")?;

		let machine_type = row
			.try_get::<String, &str>("machine_type")?
			.parse::<Uuid>()?;

		Ok(WithId::new(
			deployment_id,
			Deployment {
				name,
				image_tag,
				status,
				registry: DeploymentRegistry::ExternalRegistry {
					registry,
					image_name,
				},
				// WARN: This is a dummy runner ID, as there is no runner-id in self-hosted PATR
				runner: models::utils::Uuid::parse_str("00000000-0000-0000-0000-000000000000")
					.unwrap(),
				current_live_digest: None,
				machine_type: machine_type.into(),
			},
		))
	})
	.collect::<Result<_, ErrorType>>()?;

	AppResponse::builder()
		.body(ListDeploymentResponse { deployments })
		.headers(ListDeploymentResponseHeaders {
			total_count: TotalCountHeader(1),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
