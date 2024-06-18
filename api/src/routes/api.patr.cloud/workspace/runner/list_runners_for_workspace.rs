use axum::http::StatusCode;
use models::{api::workspace::runner::*, prelude::*};

use crate::prelude::*;

pub async fn list_runners_for_workspace(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListRunnersForWorkspacePath { workspace_id },
				query: Paginated {
					data: (),
					count,
					page,
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
		config: _,
		user_data,
	}: AuthenticatedAppRequest<'_, ListRunnersForWorkspaceRequest>,
) -> Result<AppResponse<ListRunnersForWorkspaceRequest>, ErrorType> {
	info!("Listing runners in workspace `{}`", workspace_id);

	let mut total_count = 0;

	let runners = query!(
		r#"
		SELECT
			runner.id,
            name,
			COUNT(*) OVER() AS "total_count!"
		FROM
			runner
		INNER JOIN
			RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID($2, $3) AS resource
		ON
			runner.id = resource.id
		WHERE
			workspace_id = $1 AND
			runner.deleted IS NULL
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
		Ok(WithId::new(
			row.id,
			Runner {
				name: row.name,
				connected: false, // TODO
				last_seen: None,  // TODO
			},
		))
	})
	.collect::<Result<_, ErrorType>>()?;

	AppResponse::builder()
		.body(ListRunnersForWorkspaceResponse { runners })
		.headers(ListRunnersForWorkspaceResponseHeaders {
			total_count: TotalCountHeader(total_count as _),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
