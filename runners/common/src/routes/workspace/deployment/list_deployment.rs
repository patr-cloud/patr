use http::StatusCode;
use models::{api::workspace::deployment::*, utils::Uuid};

use crate::prelude::*;

/// The handler to list all deployments in the workspace. This will return
/// all the deployments.
pub async fn list_deployment(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: ListDeploymentPath { workspace_id: _ },
				query: Paginated {
					data: (),
					count,
					page,
				},
				headers:
					ListDeploymentRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ListDeploymentRequestProcessed,
			},
		database,
		runner_changes_sender: _,
		config: _,
	}: AppRequest<'_, ListDeploymentRequest>,
) -> Result<AppResponse<ListDeploymentRequest>, ErrorType> {
	trace!("Listing all deployments");

	let rows = query(
		r#"
		SELECT
			id,
			name,
			status,
			registry,
			image_name,
			image_tag,
			machine_type,
			current_live_digest
		FROM
			deployment
		LIMIT $1 OFFSET $2;
		"#,
	)
	.bind(u32::try_from(count)?)
	.bind(u32::try_from(count * page)?)
	.fetch_all(&mut **database)
	.await?;

	let total_count = rows.len();

	let deployments = rows
		.into_iter()
		.map(|row| {
			let deployment_id = row.try_get::<Uuid, _>("id")?;
			let name = row.try_get::<String, _>("name")?;
			let status = row.try_get::<DeploymentStatus, _>("status")?;
			let registry = row.try_get::<String, _>("registry")?;
			let image_tag = row.try_get::<String, _>("image_tag")?;
			let image_name = row.try_get::<String, _>("image_name")?;
			let machine_type = row.try_get::<Uuid, _>("machine_type")?;

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
					runner: Uuid::nil(),
					current_live_digest: None,
					machine_type,
				},
			))
		})
		.collect::<Result<_, ErrorType>>()?;

	AppResponse::builder()
		.body(ListDeploymentResponse { deployments })
		.headers(ListDeploymentResponseHeaders {
			total_count: TotalCountHeader(total_count),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
