use axum::http::StatusCode;
use models::{api::workspace::managed_url::*, prelude::*};

use crate::prelude::*;

/// The handler to delete a managed URL in a workspace. This will delete the
/// managed URL and remove it from the workspace. The managed URL must be owned
/// by the user and not already deleted.
pub async fn delete_managed_url(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: DeleteManagedURLPath {
					workspace_id,
					managed_url_id,
				},
				query: (),
				headers:
					DeleteManagedURLRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: DeleteManagedURLRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, DeleteManagedURLRequest>,
) -> Result<AppResponse<DeleteManagedURLRequest>, ErrorType> {
	info!("Deleting ManagedURL `{}`", managed_url_id);

	let managed_url = query!(
		r#"
		SELECT
			managed_url.id,
			managed_url.workspace_id
		FROM
			managed_url
		INNER JOIN
			resource
		ON
			managed_url.id = resource.id
		WHERE
			managed_url.id = $1 AND
			managed_url.deleted IS NULL AND
			resource.owner_id = $2;
		"#,
		managed_url_id as _,
		workspace_id as _,
	)
	.fetch_optional(&mut **database)
	.await?;

	if let Some(managed_url) = managed_url {
		query!(
			r#"
			UPDATE
				managed_url
			SET
				deleted = NOW()
			WHERE
				id = $1;
			"#,
			managed_url.id as _,
		)
		.execute(&mut **database)
		.await?;
	}

	AppResponse::builder()
		.body(DeleteManagedURLResponse)
		.headers(())
		.status_code(StatusCode::RESET_CONTENT)
		.build()
		.into_result()
}
