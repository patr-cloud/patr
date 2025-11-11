//! DELETE blob upload cancellation endpoint handler.
//!
//! This handler cancels an ongoing blob upload session, aborting the S3
//! multipart upload and cleaning up the session from the database.

use axum::body::Body;

use crate::{
	prelude::*,
	routes::registry_patr_cloud::{
		AuthenticatedRegistryRequest,
		RegistryEndpoint,
		RegistryError,
		RegistryResponse,
		types::RepositoryName,
		utils::{repository::verify_workspace_access, s3::abort_multipart_upload},
	},
};

macros::declare_registry_endpoint!(
	/// DELETE blob upload cancellation endpoint.
	///
	/// Cancels an ongoing blob upload session.
	CancelBlobUpload,
	DELETE "/v2/{name}/blobs/uploads/{uuid}" {
		/// The repository name in the format workspace_id/repo_name
		pub name: String,
		/// The upload session UUID
		pub uuid: String,
	},
	auth = true,
	response_headers = {}
);

/// Handler for DELETE /v2/{name}/blobs/uploads/{uuid}
///
/// This handler:
/// 1. Parses and validates the repository name
/// 2. Verifies workspace access
/// 3. Retrieves upload session from database
/// 4. Aborts S3 multipart upload
/// 5. Deletes upload session from database
/// 6. Returns 204 No Content
///
/// # Requirements
/// - 6.5: Abort S3 multipart upload on cancellation
/// - 6.6: Clean up upload session
/// - 12.1: Use database transaction
pub async fn handler(
	req: AuthenticatedRegistryRequest<'_, CancelBlobUploadPath>,
) -> Result<RegistryResponse<CancelBlobUploadPath>, RegistryError> {
	info!(
		repository = %req.path.name,
		uuid = %req.path.uuid,
		user_id = %req.user_data.id,
		"DELETE blob upload cancellation request"
	);

	// 1. Parse repository name
	let repo_name = RepositoryName::parse(&req.path.name)?;
	debug!(
		workspace_id = %repo_name.workspace_id(),
		repo_name = %repo_name.name(),
		"Parsed repository name"
	);

	// 2. Verify workspace access
	verify_workspace_access(&req.user_data, repo_name.workspace_id())?;
	debug!("Workspace access verified");

	// 3. Parse UUID
	let session_id = Uuid::parse_str(&req.path.uuid)
		.map_err(|_| RegistryError::blob_upload_unknown(&req.path.uuid))?;

	// 4. Retrieve upload session from database
	#[derive(Debug)]
	struct SessionRecord {
		aws_session_id: Option<String>,
	}

	let session = sqlx::query_as!(
		SessionRecord,
		r#"
		SELECT aws_session_id
		FROM container_registry_session
		WHERE id = $1
			AND user_id = $2
		"#,
		session_id as _,
		req.user_data.id as _
	)
	.fetch_optional(&mut **req.database)
	.await?
	.ok_or_else(|| {
		warn!(
			session_id = %session_id,
			user_id = %req.user_data.id,
			"Upload session not found"
		);
		RegistryError::blob_upload_unknown(&req.path.uuid)
	})?;

	let upload_id = session.aws_session_id.ok_or_else(|| {
		error!(
			session_id = %session_id,
			"Upload session missing AWS session ID"
		);
		RegistryError::blob_upload_invalid("Upload session is not properly initialized".to_string())
	})?;

	debug!(
		session_id = %session_id,
		upload_id = %upload_id,
		"Retrieved upload session"
	);

	// 5. Use S3 bucket from request (pre-initialized in AppState)
	let bucket = req.s3_bucket;

	// 6. Construct S3 key for the upload
	let s3_key = format!("uploads/{}", session_id);

	// 7. Abort S3 multipart upload
	abort_multipart_upload(&bucket, &s3_key, &upload_id).await?;
	info!(
		session_id = %session_id,
		upload_id = %upload_id,
		s3_key = %s3_key,
		"Aborted S3 multipart upload"
	);

	// 8. Delete upload session from database
	sqlx::query!(
		r#"
		DELETE FROM container_registry_session
		WHERE id = $1
		"#,
		session_id as _
	)
	.execute(&mut **req.database)
	.await?;

	info!(
		session_id = %session_id,
		"Deleted upload session from database"
	);

	// 9. Return 204 No Content
	Ok(RegistryResponse::new(
		CancelBlobUploadResponseHeaders {},
		Body::empty(),
		http::StatusCode::NO_CONTENT,
	))
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
