use axum::http::StatusCode;
use models::{api::workspace::runner::*, prelude::*};
use rustis::commands::{GenericCommands, ScanOptions};

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
		redis,
		client_ip: _,
		config: _,
		user_data,
	}: AuthenticatedAppRequest<'_, ListRunnersForWorkspaceRequest>,
) -> Result<AppResponse<ListRunnersForWorkspaceRequest>, ErrorType> {
	info!("Listing runners in workspace `{}`", workspace_id);

	let (_, connected_runners) = redis
		.scan::<_, Vec<String>>(
			0,
			ScanOptions::default().match_pattern(redis::keys::runner_connection_lock_prefix()),
		)
		.await?;

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
		Permission::Runner(RunnerPermission::View) as _,
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
			Runner {
				name: row.name,
				connected: connected_runners
					.contains(&redis::keys::runner_connection_lock(&row.id.into())),
				last_seen: None, // TODO
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
