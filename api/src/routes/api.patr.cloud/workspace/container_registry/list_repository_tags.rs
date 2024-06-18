use axum::http::StatusCode;
use models::{api::workspace::container_registry::*, prelude::*};

use crate::prelude::*;

pub async fn list_repository_tags(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					ListContainerRepositoryTagsPath {
						workspace_id: _,
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
	info!("Listing tags for repository: {}", repository_id);

	let mut total_count = 0;
	let tags = query!(
		r#"
		SELECT
			*,
			COUNT(*) OVER() AS "count!"
		FROM
			container_registry_repository_tag
		WHERE
			repository_id = $1
		ORDER BY
			last_updated DESC
		LIMIT $2
		OFFSET $3;
		"#,
		repository_id as _,
		count as i32,
		(page * count) as i32,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		total_count = row.count;
		ContainerRepositoryTagAndDigestInfo {
			tag: row.tag,
			last_updated: row.last_updated,
			digest: row.manifest_digest,
		}
	})
	.collect();

	AppResponse::builder()
		.body(ListContainerRepositoryTagsResponse { tags })
		.headers(ListContainerRepositoryTagsResponseHeaders {
			total_count: TotalCountHeader(total_count as _),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
