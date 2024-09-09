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
			runner,
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
		let deployment_id = row.get::<String, &str>("id");
		let deployment_name = row.get::<String, &str>("name");
		let deployment_id =
			Uuid::parse_str(&deployment_id).expect("deployment id to be valid uuid");

		// TODO: either use query_as, or write row.get() for everything and convert them
		// into the correct types

		WithId::new(
			deployment_id,
			Deployment {
				name: deployment_name.to_string(),
				image_tag: "test".to_string(),
				registry: DeploymentRegistry::ExternalRegistry {
					registry: "test".to_string(),
					image_name: "test".to_string(),
				},
				status: DeploymentStatus::Created,
				runner: models::utils::Uuid::parse_str("00000000-0000-0000-0000-000000000000")
					.unwrap(),
				current_live_digest: None,
				machine_type: models::utils::Uuid::parse_str(
					"00000000-0000-0000-0000-000000000000",
				)
				.unwrap(),
			},
		)
	})
	.collect::<Vec<_>>();

	AppResponse::builder()
		.body(ListDeploymentResponse { deployments })
		.headers(ListDeploymentResponseHeaders {
			total_count: TotalCountHeader(1),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
