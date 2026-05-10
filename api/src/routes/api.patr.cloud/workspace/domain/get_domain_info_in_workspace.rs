use http::StatusCode;
use models::api::workspace::domain::*;

use crate::prelude::*;

pub async fn get_domain_info_in_workspace(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetDomainInfoInWorkspacePath {
					workspace_id: _,
					domain_id,
				},
				query: (),
				headers:
					GetDomainInfoInWorkspaceRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: GetDomainInfoInWorkspaceRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, GetDomainInfoInWorkspaceRequest>,
) -> Result<AppResponse<GetDomainInfoInWorkspaceRequest>, ErrorType> {
	info!("Starting: Get domain info in workspace");

	let workspace_domain = query!(
		r#"
		SELECT
			workspace_domain.id,
			CONCAT(name, '.', tld) AS "name!",
			is_verified,
			last_verified
		FROM
			workspace_domain
		WHERE
            id = $1 AND
			workspace_domain.deleted IS NULL;
		"#,
		domain_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.map(|row| {
		WithId::new(
			row.id,
			WorkspaceDomain {
				name: row.name,
				is_verified: row.is_verified,
				last_verified: row.last_verified,
			},
		)
	})
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	AppResponse::builder()
		.body(GetDomainInfoInWorkspaceResponse { workspace_domain })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
