use axum::{
	extract::{Path, State},
	http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
	response::IntoResponse,
};
use preprocess::Preprocessable;
use s3::Bucket;
use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[preprocess::sync]
/// The parameters that are passed in the path of the request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathParams {
	workspace_id: Uuid,
	#[preprocess(regex = "[a-z0-9]+((\\.|_|__|-+)[a-z0-9]+)*")]
	repo_name: String,
	#[preprocess(length(max = 135))]
	digest: String,
}

#[axum::debug_handler]
pub(super) async fn handle(
	method: Method,
	Path(path): Path<PathParams>,
	State(state): State<AppState>,
) -> impl IntoResponse {
	let Ok(path) = path.preprocess() else {
		return StatusCode::NOT_FOUND.into_response();
	};

	let workspace_id = path.workspace_id;
	let Ok(mut database) = state.database.begin().await else {
		return StatusCode::INTERNAL_SERVER_ERROR.into_response();
	};

	// Check if the workspace exists
	let Ok(row) = query!(
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
	.await
	else {
		return StatusCode::INTERNAL_SERVER_ERROR.into_response();
	};

	let Some(_) = row else {
		return StatusCode::NOT_FOUND.into_response();
	};

	let Ok(bucket) = Bucket::new(
		state.config.s3.bucket.as_str(),
		s3::Region::Custom {
			region: state.config.s3.region,
			endpoint: state.config.s3.endpoint,
		},
		{
			let Ok(creds) = s3::creds::Credentials::new(
				Some(&state.config.s3.key),
				Some(&state.config.s3.secret),
				None,
				None,
				None,
			) else {
				return StatusCode::INTERNAL_SERVER_ERROR.into_response();
			};
			creds
		},
	) else {
		return StatusCode::INTERNAL_SERVER_ERROR.into_response();
	};

	if matches!(method, Method::HEAD) {
		// HEAD request. head the blob from S3 and set the headers
		(
			[(
				HeaderName::from_static("Docker-Distribution-API-Version"),
				HeaderValue::from_static("registry/2.0"),
			)]
			.into_iter()
			.collect::<HeaderMap>(),
			StatusCode::OK,
		)
			.into_response()
	} else {
		// GET request. return the blob from S3
		(
			[(
				HeaderName::from_static("Docker-Distribution-API-Version"),
				HeaderValue::from_static("registry/2.0"),
			)]
			.into_iter()
			.collect::<HeaderMap>(),
			StatusCode::OK,
		)
			.into_response()
	}
}
