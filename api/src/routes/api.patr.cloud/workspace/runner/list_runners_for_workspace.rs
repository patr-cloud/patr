use axum::http::StatusCode;
use models::{api::workspace::runner::*, prelude::*};
use semver::Version;

use crate::prelude::*;

pub async fn list_runners_for_workspace(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListRunnersForWorkspacePath { workspace_id },
				query:
					ListResourceQueryProcessed {
						sort: sort_order,
						search:
							RunnerSearchParams {
								name: name_filter,
								connected: connected_filter,
								last_seen: last_seen_filter,
							},
						count,
						page,
						additional_query: (),
					},
				headers:
					ListRunnersForWorkspaceRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ListRunnersForWorkspaceRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data,
		state: _,
	}: AuthenticatedAppRequest<'_, ListRunnersForWorkspaceRequest>,
) -> Result<AppResponse<ListRunnersForWorkspaceRequest>, ErrorType> {
	info!("Listing runners in workspace `{}`", workspace_id);

	let mut total_count = 0;
	let runners = query!(
		r#"
		SELECT
			runner.id,
			name,
			is_connected,
			last_seen,
			version,
			COUNT(*) OVER() AS "total_count!"
		FROM
			runner
		INNER JOIN
			RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID($2, $3) AS resource
		ON
			runner.id = resource.id
		WHERE
			runner.workspace_id = $1 AND
			runner.deleted IS NULL AND
			($4::TEXT IS NULL OR name ILIKE '%' || $4 || '%') AND
			($5::BOOLEAN IS NULL OR is_connected = $5) AND
			($6::TIMESTAMPTZ IS NULL OR last_seen >= $6) AND
			($7::TIMESTAMPTZ IS NULL OR last_seen <= $7)
		ORDER BY
			resource.created DESC
		LIMIT $8
		OFFSET $9;
		"#,
		workspace_id as _,
		user_data.login_id as _,
		Permission::Runner(RunnerPermission::View) as _,
		name_filter,
		connected_filter,
		last_seen_filter.as_ref().map(|last_seen| last_seen.start()) as _,
		last_seen_filter.as_ref().map(|last_seen| last_seen.end()) as _,
		count as i64,
		(count * page) as i64,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		total_count = row.total_count;
		WithId::new(
			row.id,
			Runner {
				name: row.name,
				connected: row.is_connected,
				last_seen: row.last_seen,
				version: row
					.version
					.parse::<Version>()
					.unwrap_or_else(|_| Version::new(0, 0, 0)),
			},
		)
	})
	.collect();

	AppResponse::builder()
		.body(ListRunnersForWorkspaceResponse { runners })
		.headers(ListRunnersForWorkspaceResponseHeaders {
			total_count: TotalCountHeader(total_count as _),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
