use models::{api::workspace::domain::*, utils::TotalCountHeader};
use reqwest::StatusCode;

use crate::prelude::*;

pub async fn list_domains_in_workspace(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListDomainsInWorkspacePath { workspace_id },
				query:
					ListResourceQueryProcessed {
						sort: sort_order,
						search:
							WorkspaceDomainSearchParams {
								name: name_filter,
								last_verified: last_verified_filter,
								is_verified: is_verified_filter,
							},
						count,
						page,
						additional_query: (),
					},
				headers:
					ListDomainsInWorkspaceRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ListDomainsInWorkspaceRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data,
		state: _,
	}: AuthenticatedAppRequest<'_, ListDomainsInWorkspaceRequest>,
) -> Result<AppResponse<ListDomainsInWorkspaceRequest>, ErrorType> {
	info!("Listing all domains in workspace: {}", workspace_id);

	let mut total_count = 0;
	let domains = query!(
		r#"
		SELECT
			workspace_domain.id,
			CONCAT(name, '.', tld) AS "name!",
			is_verified,
			last_verified,
			COUNT(*) OVER() AS "total_count!"
		FROM
			workspace_domain
		INNER JOIN
			RESOURCES_WITH_PERMISSION_FOR_CREDENTIAL_ID($2, $3) AS resource
		ON
			workspace_domain.id = resource.id
		WHERE
			workspace_id = $1 AND
			workspace_domain.deleted IS NULL AND
			($4::TEXT IS NULL OR CONCAT(name, tld) ILIKE '%' || $4::TEXT || '%') AND
			($5::BOOLEAN IS NULL OR is_verified = $5) AND
			($6::TIMESTAMPTZ IS NULL OR last_verified >= $6) AND
			($7::TIMESTAMPTZ IS NULL OR last_verified <= $7)
		ORDER BY
			resource.created DESC
		LIMIT $8
		OFFSET $9;
		"#,
		workspace_id as _,
		user_data.login_id as _,
		Permission::Domain(DomainPermission::View) as _,
		name_filter as _,
		is_verified_filter as _,
		last_verified_filter
			.as_ref()
			.map(|last_verified_at| last_verified_at.start()) as _,
		last_verified_filter
			.as_ref()
			.map(|last_verified_at| last_verified_at.end()) as _,
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
			WorkspaceDomain {
				name: row.name,
				is_verified: row.is_verified,
				last_verified: row.last_verified,
			},
		)
	})
	.collect();

	AppResponse::builder()
		.body(ListDomainsInWorkspaceResponse { domains })
		.headers(ListDomainsInWorkspaceResponseHeaders {
			total_count: TotalCountHeader(total_count as _),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
