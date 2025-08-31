use std::fmt::Display;

use axum::{
	Json,
	Router,
	extract::Path,
	routing::{any, get},
};
use axum_extra::routing::RouterExt;
use oci_spec::distribution::{ErrorResponse, ErrorResponseBuilder};
use reqwest::{Method, StatusCode};

use crate::prelude::*;

/// Get all blob routes
mod blobs;
/// Get the status of the registry.
mod get_registry_status;
/// Get All Manifest Routes
mod manifest;
/// Get All Tag Routes
mod tags;

type Error = (StatusCode, Json<ErrorResponse>);

fn internal_server_error_response(error: impl Display) -> Error {
	error!("{error}");
	(
		StatusCode::INTERNAL_SERVER_ERROR,
		Json(ErrorResponseBuilder::default().errors([]).build().unwrap()),
	)
}

/// Setup registry routes
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.route_with_tsr("/v2", get(get_registry_status::handle))
		.nest(
			"/{workspaceId}/{repoName}",
			Router::new()
				.nest("/blobs", blobs::setup_routes(state).await)
				.nest("/manifests", manifest::setup_routes(state).await)
				.nest("/tags", tags::setup_routes(state).await),
		)
		.fallback(|Path::<String>(path), method: Method| async move {
			warn!("No route found for {method} /{path}");
			(
				StatusCode::NOT_FOUND,
				Json(ErrorResponseBuilder::default().errors([]).build().unwrap()),
			)
		})
}

/// Get the S3 object name for a blob.
fn get_s3_object_name_for_blob(blob: &str) -> String {
	format!("registry/blobs/{blob}")
}

/// Get the S3 object for a manifest
fn get_s3_object_name_for_manifest(manifest: &str) -> String {
	format!("registry/manifest/{manifest}")
}

/// Get the S3 object for a session
fn get_s3_object_name_for_session(session_id: &str) -> String {
	format!("registry/session/${session_id}")
}
