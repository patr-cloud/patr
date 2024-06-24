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
				query: Paginated {
					data: (),
					count,
					page,
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
		config: _,
		user_data: _,
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
			deployment_id = $1
		ORDER BY
			created DESC
		LIMIT $2
		OFFSET $3;
		"#,
		deployment_id as _,
		count as i32,
		(page & count) as i32
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
