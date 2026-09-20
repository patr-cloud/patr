use axum::http::StatusCode;
use models::api::workspace::secret::*;
use time::OffsetDateTime;

use crate::prelude::*;

/// Gets the metadata of a single secret: id, name, created, and last-updated.
/// The value lives in OpenBao and is never read here.
pub async fn get_secret_info(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetSecretInfoPath {
					workspace_id,
					secret_id,
				},
				query: (),
				headers: _,
				body: GetSecretInfoRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, GetSecretInfoRequest>,
) -> Result<AppResponse<GetSecretInfoRequest>, ErrorType> {
	trace!("Getting secret info for ID: `{secret_id}`");

	let secret = query!(
		r#"
		SELECT
			secret.id AS "id: Uuid",
			secret.name AS "name: String",
			resource.created AS "created: OffsetDateTime",
			secret.last_updated AS "last_updated: OffsetDateTime"
		FROM
			secret
		INNER JOIN
			resource
		ON
			secret.id = resource.id
		WHERE
			secret.id = $1 AND
			secret.workspace_id = $2 AND
			secret.deleted IS NULL;
		"#,
		secret_id as _,
		workspace_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.map(|row| {
		WithId::new(
			row.id,
			Secret {
				name: row.name,
				deployment_id: None,
				created: row.created,
				last_updated: row.last_updated,
			},
		)
	})
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	AppResponse::builder()
		.body(GetSecretInfoResponse { secret })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
