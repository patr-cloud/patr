use axum::http::StatusCode;
use models::{api::workspace::deployment::*, utils::TotalCountHeader};

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
				body,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, ListDeploymentRequest>,
) -> Result<AppResponse<ListDeploymentRequest>, ErrorType> {
	info!("Listing all deployments in workspace: {}", workspace_id);

	let mut total_count = 0;
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
			current_live_digest,
			COUNT(*) OVER() AS "total_count!"
		FROM
			deployment
		INNER JOIN
			RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID($2, $3) AS resource
		ON
			deployment.id = resource.id
		WHERE
			workspace_id = $1 AND
			status != 'deleted'
		ORDER BY
			resource.created DESC
		LIMIT $4
		OFFSET $5;
		"#,
		workspace_id as _,
		user_data.login_id as _,
		"TODO permission_name",
		count as i32,
		(count * page) as i32,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		total_count = row.total_count;
		WithId::new(
			row.id,
			Deployment {
				name: row.name,
				registry: if row.registry == PatrRegistry.to_string() {
					DeploymentRegistry::PatrRegistry {
						registry: PatrRegistry,
						repository_id: row.repository_id.unwrap().into(),
					}
				} else {
					DeploymentRegistry::ExternalRegistry {
						registry: row.registry,
						image_name: row.image_name.unwrap().into(),
					}
				},
				image_tag: row.image_tag,
				status: row.status,
				runner: row.runner.into(),
				machine_type: row.machine_type.into(),
				current_live_digest: row.current_live_digest,
			},
		)
	})
	.collect();

	AppResponse::builder()
		.body(ListDeploymentResponse { deployments })
		.headers(ListDeploymentResponseHeaders {
			total_count: TotalCountHeader(total_count as usize),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
