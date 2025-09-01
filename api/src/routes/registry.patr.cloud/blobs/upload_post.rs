use axum::{
	body::Body,
	extract::{Path, Query, State},
	http::{HeaderMap, StatusCode},
	response::IntoResponse,
};
use futures::TryStreamExt;
use oci_spec::distribution::ErrorCode;
use serde::{Deserialize, Serialize};
use tokio_util::compat::FuturesAsyncReadCompatExt;

use crate::{
	prelude::*,
	routes::registry_patr_cloud::{
		Error,
		get_s3_object_name_for_blob,
		get_s3_object_name_for_session,
		internal_server_error_response,
	},
	utils::helper::{
		check_repository,
		check_workspace,
		convert_oci_error,
		get_s3_bucket,
		preprocess_stuff,
	},
};

#[preprocess::sync]
/// The parameters that are passed in the path of the request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathParams {
	/// The workspace ID of the repository
	workspace_id: Uuid,
	/// The name of the repository
	#[preprocess(regex = r"[a-z0-9]+((\.|_|__|-+)[a-z0-9]+)*")]
	repo_name: String,
}

#[preprocess::sync]
/// The Query Parameters that are passed
#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
pub struct QueryParams {
	/// The Digest of the blob
	#[preprocess(lowercase, trim)]
	#[serde(default)]
	digest: String,
}

/// Defines the `POST` route for `/blobs/uploads`. [end-4a](https://github.com/opencontainers/distribution-spec/blob/main/spec.md#endpoints) and [end-4b](https://github.com/opencontainers/distribution-spec/blob/main/spec.md#endpoints)
#[axum::debug_handler]
pub(super) async fn handle(
	header: HeaderMap,
	Path(path): Path<PathParams>,
	Query(query): Query<QueryParams>,
	State(state): State<AppState>,
	body: Body,
) -> Result<impl IntoResponse, Error> {
	info!("Handling blob upload POST request");
	let path = preprocess_stuff(path)?;
	let query = preprocess_stuff(query)?;

	let workspace_id = path.workspace_id;
	check_workspace(workspace_id, state.clone()).await?;

	let repository_name = path.repo_name;
	check_repository(&repository_name, state.clone()).await?;

	trace!("Getting bucket details");
	let bucket = get_s3_bucket(state.config.clone())?;
	trace!("Got bucket details");
	let mut database = state
		.database
		.begin()
		.await
		.map_err(internal_server_error_response)?;

	let digest = query.digest;
	if digest.is_empty() {
		let session_id = query!(
			r#"
			INSERT INTO container_registry_session(
				id,
				user_id
			) VALUES (
			 	$1,
				$2
			) RETURNING id;
			"#,
			Uuid::new_v4() as _,
			Uuid::nil() as _,
		)
		.fetch_one(&mut *database)
		.await
		.map_err(internal_server_error_response)?
		.id;

		let s3_key = get_s3_object_name_for_session(session_id.to_string().as_str());
		let s3_session = bucket
			.initiate_multipart_upload(&s3_key, "application/octet-stream")
			.await
			.map_err(internal_server_error_response)?;

		query!(
			r#"
			UPDATE
				container_registry_session
			SET
				aws_session_id = $1
			WHERE
				id = $2
			"#,
			s3_session.upload_id as _,
			session_id as _,
		)
		.execute(&mut *database)
		.await
		.map_err(internal_server_error_response)?;

		let location = format!(
			"https://registry.patr.cloud/v2/{}/{}/blobs/uploads/{}",
			&workspace_id, &repository_name, &session_id
		);
		let headers = [
			("Docker-Distribution-API-Version", "registry/2.0"),
			("Location", location.as_str()),
		];

		return Ok((StatusCode::CREATED, headers).into_response());
	}

	let header_content_length = header
		.get("Content-Length")
		.ok_or_else(|| {
			convert_oci_error(
				StatusCode::BAD_REQUEST,
				ErrorCode::BlobUploadInvalid,
				"Content-Length header is required".to_string(),
			)
		})?
		.to_str()
		.map_err(internal_server_error_response)?;
	let header_content_type = header
		.get("Content-Type")
		.ok_or_else(|| {
			convert_oci_error(
				StatusCode::UNSUPPORTED_MEDIA_TYPE,
				ErrorCode::BlobUploadInvalid,
				"Content-Type header is required".to_string(),
			)
		})?
		.to_str()
		.map_err(internal_server_error_response)?;

	if header_content_type != "application/octet-stream" {
		return Err(convert_oci_error(
			StatusCode::BAD_REQUEST,
			ErrorCode::BlobUploadInvalid,
			format!(
				"Content-Type must be application/octet-stream, got {}",
				header_content_type
			),
		));
	}

	let mut body_stream = body
		.into_data_stream()
		.map_err(std::io::Error::other)
		.into_async_read()
		.compat();

	let s3_key = get_s3_object_name_for_blob(&digest);

	// TODO match the content length
	let status = bucket
		.put_object_stream_with_content_type(&mut body_stream, s3_key, "application/octet-stream")
		.await
		.map_err(internal_server_error_response)?;

	if !(200..300).contains(&status.status_code()) {
		return Err(convert_oci_error(
			StatusCode::BAD_REQUEST,
			ErrorCode::ManifestInvalid,
			"Failed to push manifest to S3".to_string(),
		));
	}

	query!(
		r#"
		INSERT INTO container_registry_layer_blob(
			digest,
			size
		)
		VALUES ($1, $2);
		"#,
		&digest,
		header_content_length
			.parse::<i64>()
			.map_err(internal_server_error_response)?
	)
	.execute(&mut *database)
	.await
	.map_err(internal_server_error_response)?;

	let location = format!(
		"https://registry.patr.cloud/v2/{}/{}/blobs/upload/{}",
		path.workspace_id, repository_name, digest
	);
	let headers = [
		("Docker-Distribution-API-Version", "registry/2.0"),
		("Docker-Content-Digest", &digest.to_string()),
		("Location", &location),
	];

	Ok((StatusCode::OK, headers).into_response())
}
