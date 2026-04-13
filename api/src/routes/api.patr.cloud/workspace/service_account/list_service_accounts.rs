use std::collections::BTreeMap;

use axum::http::StatusCode;
use models::{
	api::workspace::{rbac::user::RoleBindingGrant, service_account::*},
	prelude::*,
};

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
	let rows = query!(
		r#"
		SELECT
			service_account.id AS "id: Uuid",
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
	.await?;

	// One query for every grant on the page, folded by actor
	let ids = rows.iter().map(|row| row.id).collect::<Vec<_>>();
	let mut grants_by_actor = query!(
		r#"
		SELECT
			actor_id AS "actor_id: Uuid",
			role_id AS "role_id: Uuid",
			scope_id AS "scope_id: Uuid"
		FROM
			role_binding
		WHERE
			actor_id = ANY($1::UUID[]);
		"#,
		&ids as _,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.fold(BTreeMap::<Uuid, Vec<_>>::new(), |mut map, row| {
		map.entry(row.actor_id).or_default().push(RoleBindingGrant {
			role_id: row.role_id,
			resource_id: row.scope_id,
		});
		map
	});

	let service_accounts = rows
		.into_iter()
		.map(|row| {
			total_count = row.total_count;
			WithId::new(
				row.id,
				ServiceAccount {
					name: row.name,
					description: row.description,
					role_bindings: grants_by_actor.remove(&row.id).unwrap_or_default(),
				},
			)
		})
		.collect::<Vec<_>>();

	AppResponse::builder()
		.body(ListServiceAccountsResponse { service_accounts })
		.headers(ListServiceAccountsResponseHeaders {
			total_count: TotalCountHeader(total_count as _),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
