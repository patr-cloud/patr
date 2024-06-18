use axum::http::StatusCode;
use models::{
	api::{workspace::deployment::*, WithId},
	ErrorType,
};

use crate::prelude::*;

/// List deployments
///
/// #Parameters
/// - `workspace_id`: The workspace ID
///
/// #Returns
/// - `deployments`: The deployments and its details
pub async fn list_deployment(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListDeploymentPath { workspace_id },
				query: _,
				headers,
				body,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, ListDeploymentRequest>,
) -> Result<AppResponse<ListDeploymentRequest>, ErrorType> {
	info!("Starting: List deployments");

	let deployments = query!(
		r#"
		SELECT
			deployment.id,
			name,
			registry,
			repository_id,
			image_name,
			image_tag,
			status as "status: DeploymentStatus",
			runner,
			machine_type,
			current_live_digest
		FROM
			deployment
		INNER JOIN
			RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID($2, $3) AS resource
		ON
			deployment.id = resource.id
		WHERE
			workspace_id = $1 AND
			status != 'deleted';
		"#,
		workspace_id as _,
		user_data.login_id as _,
		"TODO permission_name",
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|deployment| {
		WithId::new(
			deployment.id,
			Deployment {
				name: deployment.name,
				registry: if deployment.registry == PatrRegistry.to_string() {
					DeploymentRegistry::PatrRegistry {
						registry: PatrRegistry,
						repository_id: deployment.repository_id.unwrap().into(),
					}
				} else {
					DeploymentRegistry::ExternalRegistry {
						registry: deployment.registry,
						image_name: deployment.image_name.unwrap().into(),
					}
				},
				image_tag: deployment.image_tag,
				status: deployment.status,
				runner: deployment.runner.into(),
				machine_type: deployment.machine_type.into(),
				current_live_digest: deployment.current_live_digest,
			},
		)
	})
	.collect();

	AppResponse::builder()
		.body(ListDeploymentResponse { deployments })
		.headers(ListDeploymentResponseHeaders {
			total_count: todo!(),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
