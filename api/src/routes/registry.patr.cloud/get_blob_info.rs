use axum::{
	body::Body,
	extract::{Path, State},
	http::{self, HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
	response::IntoResponse,
};
use preprocess::Preprocessable;
use s3::Bucket;
use serde::{Deserialize, Serialize};

use super::{Error, RegistryError};
use crate::prelude::*;

#[preprocess::sync]
/// The parameters that are passed in the path of the request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathParams {
	workspace_id: Uuid,
	#[preprocess(regex = r"[a-z0-9]+((\.|_|__|-+)[a-z0-9]+)*")]
	repo_name: String,
	#[preprocess(length(max = 135))]
	digest: String,
}

#[axum::debug_handler]
pub(super) async fn handle(
	method: Method,
	Path(path): Path<PathParams>,
	State(state): State<AppState>,
) -> Result<impl IntoResponse, Error> {
	let Ok(path) = path.preprocess() else {
		return Err(super::error(RegistryError::BlobUnknown, ""));
	};

	let workspace_id = path.workspace_id;
	let mut database = state.database.begin().await?;

	// Check if the workspace exists
	let row = query!(
		r#"
		SELECT
			*
		FROM
			workspace
		WHERE
			id = $1 AND
			deleted IS NULL
		"#,
		workspace_id as _
	)
	.fetch_optional(&mut *database)
	.await?;

	let Some(_) = row else {
		return Err(super::error(RegistryError::BlobUnknown, ""));
	};

	let bucket = Bucket::new(
		state.config.s3.bucket.as_str(),
		s3::Region::Custom {
			region: state.config.s3.region,
			endpoint: state.config.s3.endpoint,
		},
		{
			s3::creds::Credentials::new(
				Some(&state.config.s3.key),
				Some(&state.config.s3.secret),
				None,
				None,
				None,
			)?
		},
	)?;

	let mut headers = HeaderMap::new();
	let (head, _) = bucket.head_object(&path.digest).await?;

	headers.insert(
		HeaderName::from_static("Docker-Distribution-API-Version"),
		HeaderValue::from_static("registry/2.0"),
	);

	if let Some(accept_ranges) = head.accept_ranges {
		headers.insert(
			http::header::ACCEPT_RANGES,
			HeaderValue::from_str(&accept_ranges)?,
		);
	}
	if let Some(cache_control) = head.cache_control {
		headers.insert(
			http::header::CACHE_CONTROL,
			HeaderValue::from_str(&cache_control)?,
		);
	}
	if let Some(content_disposition) = head.content_disposition {
		headers.insert(
			http::header::CONTENT_DISPOSITION,
			HeaderValue::from_str(&content_disposition)?,
		);
	}
	if let Some(content_encoding) = head.content_encoding {
		headers.insert(
			http::header::CONTENT_ENCODING,
			HeaderValue::from_str(&content_encoding)?,
		);
	}
	if let Some(content_language) = head.content_language {
		headers.insert(
			http::header::CONTENT_LANGUAGE,
			HeaderValue::from_str(&content_language)?,
		);
	}
	if let Some(content_length) = head.content_length {
		headers.insert(
			http::header::CONTENT_LENGTH,
			HeaderValue::from_str(&content_length.to_string())?,
		);
	}
	if let Some(content_type) = head.content_type {
		headers.insert(
			http::header::CONTENT_TYPE,
			HeaderValue::from_str(&content_type)?,
		);
	}
	if let Some(e_tag) = head.e_tag {
		headers.insert(http::header::ETAG, HeaderValue::from_str(&e_tag)?);
	}
	if let Some(expires) = head.expires {
		headers.insert(http::header::EXPIRES, HeaderValue::from_str(&expires)?);
	}
	if let Some(last_modified) = head.last_modified {
		headers.insert(
			http::header::LAST_MODIFIED,
			HeaderValue::from_str(&last_modified)?,
		);
	}

	if matches!(method, Method::HEAD) {
		// HEAD request. head the blob from S3 and set the headers
		Ok((StatusCode::OK, headers).into_response())
	} else {
		// GET request. return the blob from S3
		let object = bucket.get_object_stream(path.digest).await?;
		if !(200..300).contains(&object.status_code) {
			return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
		}
		Ok((StatusCode::OK, headers, Body::from_stream(object.bytes)).into_response())
	}
}
