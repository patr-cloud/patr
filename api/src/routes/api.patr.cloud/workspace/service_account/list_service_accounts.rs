use axum::http::StatusCode;
use models::{api::workspace::service_account::*, prelude::*};

use crate::prelude::*;

pub async fn list_service_accounts(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListServiceAccountsPath { workspace_id },
				query:
					ListResourceQueryProcessed {
						sort: _sort_order,
						search:
							ServiceAccountSearchParams {
								name: name_filter,
								description: description_filter,
							},
						count,
						page,
						additional_query: (),
					},
				headers:
					ListServiceAccountsRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ListServiceAccountsRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data,
		state: _,
	}: AuthenticatedAppRequest<'_, ListServiceAccountsRequest>,
) -> Result<AppResponse<ListServiceAccountsRequest>, ErrorType> {
	info!("Listing service accounts in workspace `{}`", workspace_id);

	let mut total_count = 0;
	let mut service_accounts = query!(
		r#"
		SELECT
			service_account.id,
			service_account.name,
			service_account.description,
			COUNT(*) OVER() AS "total_count!"
		FROM
			service_account
		INNER JOIN
			RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID($2, $3) AS resource
		ON
			service_account.id = resource.id
		WHERE
			service_account.workspace_id = $1 AND
			service_account.deleted IS NULL AND
			($4::TEXT IS NULL OR service_account.name ILIKE '%' || $4 || '%') AND
			($5::TEXT IS NULL OR service_account.description ILIKE '%' || $5 || '%')
		ORDER BY
			resource.created DESC
		LIMIT $6
		OFFSET $7;
		"#,
		workspace_id as _,
		user_data.login_id as _,
		Permission::ServiceAccount(ServiceAccountPermission::View) as _,
		name_filter,
		description_filter,
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
			ServiceAccount {
				name: row.name,
				description: row.description,
				roles: Vec::new(), // Roles populated below
			},
		)
	})
	.collect::<Vec<_>>();

	// Populate roles for each service account
	for service_account in &mut service_accounts {
		service_account.data.roles = query!(
			r#"
			SELECT
				role_id AS "role_id: Uuid"
			FROM
				service_account_role
			WHERE
				service_account_id = $1;
			"#,
			service_account.id as _,
		)
		.fetch_all(&mut **database)
		.await?
		.into_iter()
		.map(|row| row.role_id)
		.collect::<Vec<_>>();
	}

	AppResponse::builder()
		.body(ListServiceAccountsResponse { service_accounts })
		.headers(ListServiceAccountsResponseHeaders {
			total_count: TotalCountHeader(total_count as _),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
