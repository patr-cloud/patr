use axum::http::StatusCode;
use models::{api::workspace::rbac::role::*, utils::TotalCountHeader};

use crate::prelude::*;

pub async fn list_all_roles(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListAllRolesPath { workspace_id },
				query: Paginated {
					data: (),
					count,
					page,
				},
				headers:
					ListAllRolesRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ListAllRolesRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, ListAllRolesRequest>,
) -> Result<AppResponse<ListAllRolesRequest>, ErrorType> {
	info!("Listing all roles for workspace: {}", workspace_id);

	let mut total_count = 0;
	let roles = query!(
		r#"
        SELECT
            *,
            COUNT(*) OVER() AS "total_count!"
        FROM
            role
        WHERE
            owner_id = $1
        LIMIT $2
        OFFSET $3;
        "#,
		workspace_id as _,
		count as i64,
		(page * count) as i64,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		total_count = row.total_count;
		WithId::new(
			row.id,
			Role {
				name: row.name,
				description: row.description,
			},
		)
	})
	.collect();

	AppResponse::builder()
		.body(ListAllRolesResponse { roles })
		.headers(ListAllRolesResponseHeaders {
			total_count: TotalCountHeader(total_count as usize),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
