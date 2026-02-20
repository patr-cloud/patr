//! GET blob upload status endpoint handler.
//!
//! This handler retrieves the status of an ongoing blob upload session,
//! returning the current byte range that has been uploaded.

use std::str::FromStr;

use axum::body::Body;
use rustis::commands::StringCommands;

use crate::{redis::keys, routes::registry_patr_cloud::prelude::*};

macros::declare_registry_endpoint!(
	/// GET blob upload status endpoint.
	///
	/// Retrieves the status of an ongoing blob upload session.
	GetBlobUploadStatus,
	GET "/v2/{workspace_id}/{repo_name}/blobs/uploads/{session_id}" {
		/// The workspace ID
		pub workspace_id: Uuid,
		/// The repository name
		#[preprocess(lowercase, regex = constants::REGISTRY_REPO_NAME_REGEX, length(max = 255))]
		pub repo_name: String,
		/// The upload session UUID
		pub session_id: Uuid,
	},
	request_headers = {
		/// The authorization header
		pub authorization: BearerToken,
	},
	response_headers = {
		/// The current byte range that has been uploaded
		pub range: Range,
		/// Where to upload the next chunk
		pub location: Location,
	}
);

/// Handler for GET /v2/{workspace_id}/{repo_name}/blobs/uploads/{session_id}
///
/// This handler:
/// - Verifies workspace access
/// - Retrieves upload session from redis
/// - Returns 204 No Content with Range and Location headers
pub async fn get_upload_status(
	AuthenticatedRegistryAppRequest {
		request:
			RegistryProcessedApiRequest {
				path:
					GetBlobUploadStatusPathProcessed {
						workspace_id,
						repo_name,
						session_id,
					},
				query: (),
				headers: GetBlobUploadStatusRequestHeaders { authorization: _ },
				body: _,
			},
		database,
		redis,
		s3: _,
		client_ip: _,
		user_data,
		config: _,
	}: AuthenticatedRegistryAppRequest<'_, GetBlobUploadStatusPath>,
) -> Result<RegistryResponse<GetBlobUploadStatusPath>, RegistryError> {
	info!("GET blob upload status request");

	// Check that the user can push to this repository
	let (repository_id, permission_id) = query!(
		r#"
		SELECT
			id AS "resource_id: Uuid",
			(
				SELECT
					id
				FROM
					permission
				WHERE
					name = $3
			) AS "permission_id!: Uuid"
		FROM
			container_registry_repository
		WHERE
			workspace_id = $1 AND
			name = $2 AND
			deleted IS NULL;
		"#,
		workspace_id as _,
		&repo_name,
		Permission::ContainerRegistryRepository(ContainerRegistryRepositoryPermission::Push)
			.to_string(),
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or_else(|| {
		warn!("Repository `{workspace_id}/{repo_name}` not found");
		RegistryError::builder()
			.status(StatusCode::NOT_FOUND)
			.message("Repository not found")
			.code(ErrorCode::NameUnknown)
			.build()
	})
	.map(|row| (row.resource_id, row.permission_id))?;

	let authorized =
		user_data.has_permission_on_resource(workspace_id, repository_id, permission_id);

	if !authorized {
		// Intentionally return a 404 to avoid leaking repository existence
		debug!("User not authorized to access repository");
		return RegistryError::builder()
			.status(StatusCode::NOT_FOUND)
			.message("Repository not found")
			.code(ErrorCode::NameUnknown)
			.build()
			.into_result();
	}

	let session = serde_json::from_str::<S3UploadSession>(
		&redis
			.get::<String>(keys::registry_blob_upload_part_prefix(&session_id))
			.await?,
	)?;

	// Return 204 No Content with Range and Location headers
	RegistryResponse::builder()
		.status_code(StatusCode::NO_CONTENT)
		.headers(GetBlobUploadStatusResponseHeaders {
			range: Range::new(0..session.total_bytes_uploaded).map_err(|err| {
				error!("Invalid range error: {}", err);
				RegistryError::builder()
					.code(ErrorCode::SizeInvalid)
					.message(
						if cfg!(debug_assertions) {
							format!("invalid range specified: {}", err)
						} else {
							"invalid range specified".to_string()
						},
					)
					.status(StatusCode::INTERNAL_SERVER_ERROR)
					.build()
			})?,
			location: Location::from_str(&format!(
				"/v2/{workspace_id}/{repo_name}/blobs/uploads/{session_id}"
			))?,
		})
		.body(Body::empty())
		.build()
		.into_result()
}
