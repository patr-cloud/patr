use axum::http::StatusCode;
use models::{api::workspace::container_registry::*, prelude::*};

use crate::prelude::*;

pub async fn list_repository_tags(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListContainerRepositoryTagsPath {
					workspace_id,
					repository_id,
				},
				query: Paginated {
					page,
					count,
					data: (),
				},
				headers:
					ListContainerRepositoryTagsRequestHeaders {
						user_agent: _,
						authorization: _,
					},
				body: ListContainerRepositoryTagsRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, ListContainerRepositoryTagsRequest>,
) -> Result<AppResponse<ListContainerRepositoryTagsRequest>, ErrorType> {
	info!("Starting: List repository tags");

	let tags = query!(
		r#"
		SELECT
			tag,
			manifest_digest,
			last_updated
		FROM
			container_registry_repository_tag
		WHERE
			repository_id = $1;
		"#,
		repository_id as _
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| ContainerRepositoryTagAndDigestInfo {
		tag: row.tag,
		last_updated: row.last_updated.into(),
		digest: row.manifest_digest,
	})
	.collect();

	let total_count = query!(
		r#" 
		SELECT
			COUNT(*) AS count
		FROM
			container_registry_repository_tag
		WHERE
			repository_id = $1;
		"#,
		repository_id as _
	)
	.fetch_one(&mut **database)
	.await
	.map(|repo| repo.count)?
	.ok_or(ErrorType::server_error(
		"Failed to get total repository count",
	))?;

	AppResponse::builder()
		.body(ListContainerRepositoryTagsResponse { tags })
		.headers(ListContainerRepositoryTagsResponseHeaders {
			total_count: TotalCountHeader(total_count as usize),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
