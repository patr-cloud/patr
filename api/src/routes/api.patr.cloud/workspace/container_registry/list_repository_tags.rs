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
				query:
					ListResourceQueryProcessed {
						sort: sort_order,
						search:
							ContainerRepositoryTagAndDigestInfoSearchParams {
								tag: tag_filter,
								digest: digest_filter,
								last_updated: last_updated_filter,
							},
						count,
						page,
						additional_query: (),
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
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, ListContainerRepositoryTagsRequest>,
) -> Result<AppResponse<ListContainerRepositoryTagsRequest>, ErrorType> {
	info!("Listing tags for repository: {}", repository_id);

	let mut total_count = 0;
	let tags = query!(
		r#"
		SELECT
			name,
			last_updated,
			manifest_digest,
			COUNT(*) OVER() AS "count!"
		FROM
			container_registry_repository_tag
		WHERE
			repository_id = $1 AND
			($2::TEXT IS NULL OR name ILIKE '%' || $2 || '%') AND
			($3::TEXT IS NULL OR manifest_digest ILIKE '%' || $3 || '%') AND
			($4::TIMESTAMPTZ IS NULL OR last_updated >= $4) AND
			($5::TIMESTAMPTZ IS NULL OR last_updated <= $5)
		ORDER BY
			last_updated DESC
		LIMIT $6
		OFFSET $7;
		"#,
		repository_id as _,
		tag_filter,
		digest_filter,
		last_updated_filter.as_ref().map(|range| range.start()),
		last_updated_filter.as_ref().map(|range| range.end()),
		count as i32,
		(page * count) as i32,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		total_count = row.count;
		ContainerRepositoryTagAndDigestInfo {
			tag: row.name,
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
