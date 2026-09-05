use axum::http::StatusCode;
use models::{api::workspace::deployment::*, utils::TotalCountHeader};

use crate::prelude::*;

/// The handler to list all deployments in the workspace. This will return
/// all the deployments in the workspace.
pub async fn list_deployment(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListDeploymentPath { workspace_id },
				query:
					ListResourceQueryProcessed {
						sort: sort_order,
						search:
							DeploymentSearchParams {
								name: name_filter,
								// TODO nested search params
								registry: registry_filter,
								image_tag: image_tag_filter,
								status: status_filter,
								runner: runner_filter,
								current_live_digest: current_live_digest_filter,
							},
						count,
						page,
						additional_query: (),
					},
				headers:
					ListDeploymentRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ListDeploymentRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data,
		state: _,
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
			status AS "status: DeploymentStatus",
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
			deployment.workspace_id = $1 AND
			deployment.deleted IS NULL AND
			($4::TEXT IS NULL OR deployment.name ILIKE '%' || $4 || '%') AND
			($5::TEXT IS NULL OR deployment.image_tag = $5) AND
			(
				$6::DEPLOYMENT_STATUS[] IS NULL OR
				deployment.status = ANY($6)
			) AND
			($7::UUID IS NULL OR deployment.runner = $7) AND
			($8::TEXT IS NULL OR deployment.current_live_digest = $8)
		ORDER BY
			resource.created DESC
		LIMIT $9
		OFFSET $10;
		"#,
		workspace_id as _,
		user_data.login_id as _,
		Permission::Deployment(DeploymentPermission::View) as _,
		name_filter as _,
		image_tag_filter as _,
		status_filter as _,
		runner_filter as _,
		current_live_digest_filter as _,
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
						image_name: row.image_name.unwrap(),
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

	if page != 0 && total_count == 0 {
		return Err(ErrorType::PageOutOfBounds);
	}

	AppResponse::builder()
		.body(ListDeploymentResponse { deployments })
		.headers(ListDeploymentResponseHeaders {
			total_count: TotalCountHeader(total_count as _),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
