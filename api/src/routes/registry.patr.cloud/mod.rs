use std::fmt::Display;

use axum::{Json, Router, routing::get};
use oci_spec::distribution::{ErrorResponse, ErrorResponseBuilder};
use reqwest::StatusCode;

use crate::prelude::*;

/// Get all blob routes
mod blobs;
/// Download a specific blob, given its digest.
mod get_blob_info;
/// Get the manifest for a specific reference.
mod get_manifest_info;
/// Get the status of the registry.
mod get_registry_status;
/// Get All Manifest Routes
mod manifest;

type Error = (StatusCode, Json<ErrorResponse>);

fn internal_server_error_response(error: impl Display) -> Error {
	error!("{error}");
	(
		StatusCode::INTERNAL_SERVER_ERROR,
		Json(ErrorResponseBuilder::default().errors([]).build().unwrap()),
	)
}

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.nest(
			"/v2",
			Router::new()
				.route("/v2", get(get_registry_status::handle))
				.nest(
					"/:workspaceId/:repoName",
					Router::new().nest("/blobs", blobs::setup_routes(state).await),
				),
		)
		.with_state(())
}

/// Get the S3 object name for a blob.
fn get_s3_object_name_for_blob(blob: &str) -> String {
	format!("registry/blobs/{blob}")
}

// .route(
// 	"/:workspaceId/:repoName/blobs/:digest",
// 	get(get_blob_info::handle).head(get_blob_info::handle),
// )
// .route(
// 	"/:workspaceId/:repoName/manifests/:reference",
// 	get(get_manifest_info::handle),
// ),
