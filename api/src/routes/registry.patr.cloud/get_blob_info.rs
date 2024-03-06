use axum::{
	body::Body,
	extract::{Path, State},
	http::{self, HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
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
		let Ok((head, _)) = bucket.head_object(path.digest).await else {
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		};
		(StatusCode::OK, {
			let mut headers = HeaderMap::new();

			headers.insert(
				HeaderName::from_static("Docker-Distribution-API-Version"),
				HeaderValue::from_static("registry/2.0"),
			);

			if let Some(accept_ranges) = head.accept_ranges {
				headers.insert(http::header::ACCEPT_RANGES, {
					let Ok(value) = HeaderValue::from_str(&accept_ranges) else {
						return StatusCode::INTERNAL_SERVER_ERROR.into_response();
					};
					value
				});
			}
			if let Some(cache_control) = head.cache_control {
				headers.insert(http::header::CACHE_CONTROL, {
					let Ok(value) = HeaderValue::from_str(&cache_control) else {
						return StatusCode::INTERNAL_SERVER_ERROR.into_response();
					};
					value
				});
			}
			if let Some(content_disposition) = head.content_disposition {
				headers.insert(http::header::CONTENT_DISPOSITION, {
					let Ok(value) = HeaderValue::from_str(&content_disposition) else {
						return StatusCode::INTERNAL_SERVER_ERROR.into_response();
					};
					value
				});
			}
			if let Some(content_encoding) = head.content_encoding {
				headers.insert(http::header::CONTENT_ENCODING, {
					let Ok(value) = HeaderValue::from_str(&content_encoding) else {
						return StatusCode::INTERNAL_SERVER_ERROR.into_response();
					};
					value
				});
			}
			if let Some(content_language) = head.content_language {
				headers.insert(http::header::CONTENT_LANGUAGE, {
					let Ok(value) = HeaderValue::from_str(&content_language) else {
						return StatusCode::INTERNAL_SERVER_ERROR.into_response();
					};
					value
				});
			}
			if let Some(content_length) = head.content_length {
				headers.insert(http::header::CONTENT_LENGTH, {
					let Ok(value) = HeaderValue::from_str(&content_length.to_string()) else {
						return StatusCode::INTERNAL_SERVER_ERROR.into_response();
					};
					value
				});
			}
			if let Some(content_type) = head.content_type {
				headers.insert(http::header::CONTENT_TYPE, {
					let Ok(value) = HeaderValue::from_str(&content_type) else {
						return StatusCode::INTERNAL_SERVER_ERROR.into_response();
					};
					value
				});
			}
			if let Some(e_tag) = head.e_tag {
				headers.insert(http::header::ETAG, {
					let Ok(value) = HeaderValue::from_str(&e_tag) else {
						return StatusCode::INTERNAL_SERVER_ERROR.into_response();
					};
					value
				});
			}
			if let Some(expires) = head.expires {
				headers.insert(http::header::EXPIRES, {
					let Ok(value) = HeaderValue::from_str(&expires) else {
						return StatusCode::INTERNAL_SERVER_ERROR.into_response();
					};
					value
				});
			}
			if let Some(last_modified) = head.last_modified {
				headers.insert(http::header::LAST_MODIFIED, {
					let Ok(value) = HeaderValue::from_str(&last_modified) else {
						return StatusCode::INTERNAL_SERVER_ERROR.into_response();
					};
					value
				});
			}

			headers
		})
			.into_response()
	} else {
		// GET request. return the blob from S3
		let Ok((head, _)) = bucket.head_object(&path.digest).await else {
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		};
		let Ok(object) = bucket.get_object_stream(path.digest).await else {
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		};
		if !(200..300).contains(&object.status_code) {
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		}
		(
			StatusCode::OK,
			{
				let mut headers = HeaderMap::new();

				headers.insert(
					HeaderName::from_static("Docker-Distribution-API-Version"),
					HeaderValue::from_static("registry/2.0"),
				);

				if let Some(accept_ranges) = head.accept_ranges {
					headers.insert(http::header::ACCEPT_RANGES, {
						let Ok(value) = HeaderValue::from_str(&accept_ranges) else {
							return StatusCode::INTERNAL_SERVER_ERROR.into_response();
						};
						value
					});
				}
				if let Some(cache_control) = head.cache_control {
					headers.insert(http::header::CACHE_CONTROL, {
						let Ok(value) = HeaderValue::from_str(&cache_control) else {
							return StatusCode::INTERNAL_SERVER_ERROR.into_response();
						};
						value
					});
				}
				if let Some(content_disposition) = head.content_disposition {
					headers.insert(http::header::CONTENT_DISPOSITION, {
						let Ok(value) = HeaderValue::from_str(&content_disposition) else {
							return StatusCode::INTERNAL_SERVER_ERROR.into_response();
						};
						value
					});
				}
				if let Some(content_encoding) = head.content_encoding {
					headers.insert(http::header::CONTENT_ENCODING, {
						let Ok(value) = HeaderValue::from_str(&content_encoding) else {
							return StatusCode::INTERNAL_SERVER_ERROR.into_response();
						};
						value
					});
				}
				if let Some(content_language) = head.content_language {
					headers.insert(http::header::CONTENT_LANGUAGE, {
						let Ok(value) = HeaderValue::from_str(&content_language) else {
							return StatusCode::INTERNAL_SERVER_ERROR.into_response();
						};
						value
					});
				}
				if let Some(content_length) = head.content_length {
					headers.insert(http::header::CONTENT_LENGTH, {
						let Ok(value) = HeaderValue::from_str(&content_length.to_string()) else {
							return StatusCode::INTERNAL_SERVER_ERROR.into_response();
						};
						value
					});
				}
				if let Some(content_type) = head.content_type {
					headers.insert(http::header::CONTENT_TYPE, {
						let Ok(value) = HeaderValue::from_str(&content_type) else {
							return StatusCode::INTERNAL_SERVER_ERROR.into_response();
						};
						value
					});
				}
				if let Some(e_tag) = head.e_tag {
					headers.insert(http::header::ETAG, {
						let Ok(value) = HeaderValue::from_str(&e_tag) else {
							return StatusCode::INTERNAL_SERVER_ERROR.into_response();
						};
						value
					});
				}
				if let Some(expires) = head.expires {
					headers.insert(http::header::EXPIRES, {
						let Ok(value) = HeaderValue::from_str(&expires) else {
							return StatusCode::INTERNAL_SERVER_ERROR.into_response();
						};
						value
					});
				}
				if let Some(last_modified) = head.last_modified {
					headers.insert(http::header::LAST_MODIFIED, {
						let Ok(value) = HeaderValue::from_str(&last_modified) else {
							return StatusCode::INTERNAL_SERVER_ERROR.into_response();
						};
						value
					});
				}

				headers
			},
			Body::from_stream(object.bytes),
		)
			.into_response()
	}
}
