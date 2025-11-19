use axum::http::StatusCode;
use models::{api::workspace::managed_url::*, prelude::*};

use crate::prelude::*;

/// The handler to delete a managed URL in a workspace. This will delete the
/// managed URL and remove it from the workspace. The managed URL must be owned
/// by the user and not already deleted.
#[instrument(skip(database, config))]
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
		config,
		user_data: _,
	}: AuthenticatedAppRequest<'_, DeleteManagedURLRequest>,
) -> Result<AppResponse<DeleteManagedURLRequest>, ErrorType> {
	info!("Deleting ManagedURL `{}`", managed_url_id);

	let managed_url = query!(
		r#"
		WITH deleted AS (
			DELETE FROM
				managed_url
			WHERE
				id = $1
			RETURNING
				sub_domain,
				domain_id,
				path
		)
		SELECT
			deleted.sub_domain,
			CONCAT(
				workspace_domain.name,
				'.',
				workspace_domain.tld
			) AS "domain!",
			deleted.path
		FROM
			deleted
		INNER JOIN
			workspace_domain
		ON
			deleted.domain_id = workspace_domain.id;
		"#,
		managed_url_id as _,
	)
	.fetch_one(&mut **database)
	.await
	.map_err(|e| match e {
		sqlx::Error::Database(dbe) if dbe.is_foreign_key_violation() => ErrorType::ResourceInUse,
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
		managed_url_id as _,
	)
	.execute(&mut **database)
	.await?;

	super::sync_worker_kv_for_domain(
		&format!("{}.{}", managed_url.sub_domain, managed_url.domain),
		&mut **database,
		&config,
	)
	.await?;

	AppResponse::builder()
		.body(DeleteManagedURLResponse)
		.headers(())
		.status_code(StatusCode::RESET_CONTENT)
		.build()
		.into_result()
}
