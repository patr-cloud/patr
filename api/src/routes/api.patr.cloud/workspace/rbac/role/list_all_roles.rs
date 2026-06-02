use axum::http::StatusCode;
use models::{api::workspace::rbac::role::*, utils::TotalCountHeader};

use crate::prelude::*;

/// The handler to list all roles in the workspace. This will return all the
/// roles that are available in the workspace, not just the roles of the user.
/// To get the roles of the user, use the [`get_current_permissions`][1] route.
///
/// [1]: models::api::workspace::rbac::GetCurrentPermissionsRequest
pub async fn list_all_roles(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListAllRolesPath { workspace_id },
				query:
					ListResourceQueryProcessed {
						sort: sort_order,
						search:
							RoleSearchParams {
								name: name_filter,
								description: description_filter,
							},
						count,
						page,
						additional_query: (),
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
		user_data: _,
		state: _,
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
			owner_id = $1 AND
			($2::TEXT IS NULL OR name ILIKE '%' || $2::TEXT || '%') AND
			($3::TEXT IS NULL OR description ILIKE '%' || $3::TEXT || '%')
		ORDER BY
			id
		LIMIT $4
		OFFSET $5;
		"#,
		workspace_id as _,
		name_filter,
		description_filter,
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
			total_count: TotalCountHeader(total_count as _),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
