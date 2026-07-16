//! GET blob upload status endpoint handler.
//!
//! This handler retrieves the status of an ongoing blob upload session,
//! returning the current byte range that has been uploaded.

use std::str::FromStr;

use axum::body::Body;
use base64::prelude::*;
use rustis::commands::StringCommands;

use crate::{models::permissions, redis::keys, routes::registry_patr_cloud::prelude::*};

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
	query = {
		/// Ignored. A client recovering from an out-of-order chunk may reuse the
		/// URL it built for a `PUT` — which carries a `?digest=` — for the
		/// following status `GET`. Accepting (and ignoring) it keeps the strict
		/// query parser from rejecting an otherwise valid status request.
		pub digest: Option<String>,
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
				query: GetBlobUploadStatusQueryProcessed { digest: _ },
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
	let repository_id = query!(
		r#"
		SELECT
			id AS "resource_id: Uuid"
		FROM
			container_registry_repository
		WHERE
			workspace_id = $1 AND
			name = $2 AND
			deleted IS NULL;
		"#,
		workspace_id as _,
		&repo_name,
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
	.map(|row| row.resource_id)?;

	let permission_id = permissions::get_permission_id(
		database,
		Permission::ContainerRegistryRepository(ContainerRegistryRepositoryPermission::Push),
	)
	.await;

	let authorized =
		user_data.has_permission_on_resource(workspace_id, repository_id, permission_id);

	if !authorized {
		debug!("User lacks push access to repository");
		// Workspace members get a clear 403 (they can already list repos via the
		// API, so there's nothing to hide); non-members get a 404 so outsiders
		// can't enumerate private repositories.
		return if user_data.permissions.contains_key(&workspace_id) {
			RegistryError::builder()
				.status(StatusCode::FORBIDDEN)
				.message(format!(
					"You do not have push access to `{workspace_id}/{repo_name}`"
				))
				.code(ErrorCode::Denied)
				.build()
		} else {
			RegistryError::builder()
				.status(StatusCode::NOT_FOUND)
				.message("Repository not found")
				.code(ErrorCode::NameUnknown)
				.build()
		}
		.into_result();
	}

	let session = serde_json::from_str::<S3UploadSession>(
		&redis
			.get::<String>(keys::registry_blob_upload_part_prefix(&session_id))
			.await?,
	)?;

	// Bytes received so far = what's been flushed to S3 (`total_bytes_uploaded`)
	// plus any sub-threshold tail still sitting in the pending buffer. For a
	// small blob nothing is ever flushed, so the pending buffer holds everything
	// — reporting only `total_bytes_uploaded` here would wrongly say `0-0`.
	let pending_size = redis
		.get::<Option<String>>(keys::registry_blob_upload_pending_buffer(&session_id))
		.await?
		.map(|encoded| {
			BASE64_STANDARD
				.decode(&encoded)
				.map(|bytes| bytes.len() as u64)
		})
		.transpose()
		.map_err(|err| {
			error!("Failed to decode pending buffer from Redis: {err}");
			RegistryError::server_error(
				ErrorCode::BlobUploadInvalid,
				"Corrupted pending upload buffer",
			)
		})?
		.unwrap_or_default();
	let received_so_far = session.total_bytes_uploaded + pending_size;

	// Return 204 No Content with Range and Location headers
	RegistryResponse::builder()
		.status_code(StatusCode::NO_CONTENT)
		.headers(GetBlobUploadStatusResponseHeaders {
			range: Range::new(0..received_so_far).map_err(|err| {
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
