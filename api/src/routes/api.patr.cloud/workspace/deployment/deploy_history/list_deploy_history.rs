use axum::http::StatusCode;
use models::{api::workspace::deployment::deploy_history::*, utils::TotalCountHeader};

use crate::prelude::*;

/// List a deployment's history of deploys. This includes the image digest and
/// the time it was deployed.
pub async fn list_deploy_history(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListDeploymentDeployHistoryPath {
					workspace_id,
					deployment_id,
				},
				query:
					ListResourceQueryProcessed {
						sort: sort_order,
						search:
							DeploymentDeployHistorySearchParams {
								image_digest: image_digest_filter,
								created: created_filter,
							},
						count,
						page,
						additional_query: (),
					},
				headers:
					ListDeploymentDeployHistoryRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ListDeploymentDeployHistoryRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, ListDeploymentDeployHistoryRequest>,
) -> Result<AppResponse<ListDeploymentDeployHistoryRequest>, ErrorType> {
	info!("Listing deployment history");

	// Check if deployment exists
	query!(
		r#"
		SELECT
			id
		FROM
			deployment
		WHERE
			id = $1 AND
			workspace_id = $2 AND
			deleted IS NULL;
		"#,
		deployment_id as _,
		workspace_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	let mut total_count = 0;
	let deploys = query!(
		r#"
		SELECT 
			image_digest,
			created,
			COUNT(*) OVER() AS "total_count!"
		FROM
			deployment_deploy_history
		WHERE
			deployment_id = $1 AND
			($2::TEXT IS NULL OR image_digest = $2) AND
			($3::TIMESTAMPTZ IS NULL OR created >= $3) AND
			($4::TIMESTAMPTZ IS NULL OR created <= $4)
		ORDER BY
			created DESC
		LIMIT $5
		OFFSET $6;
		"#,
		deployment_id as _,
		image_digest_filter as _,
		created_filter.as_ref().map(|created_at| created_at.start()) as _,
		created_filter.as_ref().map(|created_at| created_at.end()) as _,
		count as i32,
		(page * count) as i32
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		total_count = row.total_count;
		DeploymentDeployHistory {
			image_digest: row.image_digest,
			created: row.created,
		}
	})
	.collect();

	AppResponse::builder()
		.body(ListDeploymentDeployHistoryResponse { deploys })
		.headers(ListDeploymentDeployHistoryResponseHeaders {
			total_count: TotalCountHeader(total_count as _),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
