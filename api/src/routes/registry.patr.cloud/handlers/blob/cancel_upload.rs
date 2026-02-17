//! DELETE blob upload cancellation endpoint handler.
//!
//! This handler cancels an ongoing blob upload session, aborting the S3
//! multipart upload and cleaning up the session from the database.

use axum::body::Body;
use rustis::commands::GenericCommands;

use crate::{redis::keys, routes::registry_patr_cloud::prelude::*};

macros::declare_registry_endpoint!(
	/// DELETE blob upload cancellation endpoint.
	///
	/// Cancels an ongoing blob upload session.
	CancelBlobUpload,
	DELETE "/v2/{workspace_id}/{repo_name}/blobs/uploads/{session_id}" {
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
	response_headers = {}
);

/// Handler for DELETE /v2/{workspace_id}/{repo_name}/blobs/uploads/{session_id}
///
/// This handler:
/// - Verifies workspace access
/// - Retrieves upload session from redis
/// - Aborts S3 multipart upload
/// - Deletes upload session from redis
/// - Returns 204 No Content
pub async fn cancel_upload(
	AuthenticatedRegistryAppRequest {
		request:
			RegistryProcessedApiRequest {
				path:
					CancelBlobUploadPathProcessed {
						workspace_id,
						repo_name,
						session_id,
					},
				query: (),
				headers: CancelBlobUploadRequestHeaders { authorization: _ },
				body: _,
			},
		database,
		redis,
		s3,
		client_ip: _,
		user_data,
		config,
	}: AuthenticatedRegistryAppRequest<'_, CancelBlobUploadPath>,
) -> Result<RegistryResponse<CancelBlobUploadPath>, RegistryError> {
	info!("DELETE blob upload cancellation request");

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

	debug!("Retrieved upload session");

	s3.abort_multipart_upload()
		.bucket(&config.s3.bucket)
		.upload_id(format!("uploads/{session_id}"))
		.send()
		.await?;

	info!("Aborted S3 multipart upload");

	// Delete upload session from redis
	redis
		.del(keys::registry_blob_upload_part_prefix(&session_id))
		.await?;

	// Clean up any pending buffer
	let _ = redis
		.del(keys::registry_blob_upload_pending_buffer(&session_id))
		.await;

	info!("Deleted upload session from redis");

	// 9. Return 204 No Content
	RegistryResponse::builder()
		.status_code(StatusCode::NO_CONTENT)
		.headers(CancelBlobUploadResponseHeaders {})
		.body(Body::empty())
		.build()
		.into_result()
}

/// Helper function to create an S3 bucket client from configuration.
///
/// This function creates a properly configured S3 bucket client using the
/// credentials and settings from the application configuration.
///
/// # Arguments
///
/// * `config` - The S3 configuration containing credentials and bucket details
///
/// # Returns
///
/// A boxed S3 Bucket client ready for use
#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_cancel_blob_upload_endpoint_path() {
		// Verify the endpoint path is correct
		assert_eq!(
			<CancelBlobUploadPath as axum_extra::routing::TypedPath>::PATH,
			"/v2/{name}/blobs/uploads/{uuid}"
		);
	}
}
