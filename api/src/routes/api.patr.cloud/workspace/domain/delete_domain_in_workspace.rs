use models::api::workspace::domain::*;
use reqwest::StatusCode;

use crate::prelude::*;

pub async fn delete_domain_in_workspace(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: DeleteDomainInWorkspacePath {
					workspace_id,
					domain_id,
				},
				query: (),
				headers:
					DeleteDomainInWorkspaceRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: DeleteDomainInWorkspaceRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, DeleteDomainInWorkspaceRequest>,
) -> Result<AppResponse<DeleteDomainInWorkspaceRequest>, ErrorType> {
	info!("Deleting domain `{domain_id}` in workspace `{workspace_id}`");

	// This will fail with ResourceInUse if managed URLs (or their custom
	// hostnames) still reference this domain. The user must delete all managed
	// URLs first — doing so automatically cleans up the CF custom hostnames.
	query!(
		r#"
		DELETE FROM
			workspace_domain
		WHERE
			id = $1;
		"#,
		domain_id as _
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(err) if err.is_foreign_key_violation() => ErrorType::ResourceInUse,
		err => ErrorType::server_error(err),
	})?;

	query!(
		r#"
		UPDATE
			resource
		SET
			deleted = NOW()
		WHERE
			id = $1;
		"#,
		domain_id as _
	)
	.execute(&mut **database)
	.await?;

	AppResponse::builder()
		.body(DeleteDomainInWorkspaceResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
