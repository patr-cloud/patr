use axum::{
	Json,
	extract::{Path, State},
	http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
	response::IntoResponse,
};
use oci_spec::distribution::TagListBuilder;
use serde::{Deserialize, Serialize};

use crate::{
	prelude::*,
	routes::registry_patr_cloud::{Error, internal_server_error_response},
	utils::helper::{check_workspace, preprocess_stuff},
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

/// Handles the `GET /v2/<name>/tags/list` route. [`end-8a`](https://github.com/opencontainers/distribution-spec/blob/main/spec.md#listing-tags)
/// TODO: implement the pagination
/// [`end-8b`](https://github.com/opencontainers/distribution-spec/blob/main/spec.md#listing-tags)
#[axum::debug_handler]
pub(super) async fn handle(
	Path(path): Path<PathParams>,
	State(state): State<AppState>,
) -> Result<impl IntoResponse, Error> {
	let path = preprocess_stuff(path)?;
	let repo_name = path.repo_name;

	let workspace_id = path.workspace_id;
	check_workspace(workspace_id, state.clone()).await?;

	let mut database = state
		.database
		.begin()
		.await
		.map_err(internal_server_error_response)?;

	let row = query!(
		r#"
		SELECT 
			tags.name
		FROM 
			container_registry_tag AS tags
		INNER JOIN
			container_registry_repository AS repo
		ON
			tags.repository_id = repo.id
		WHERE 
			repo.workspace_id = $1 AND
			repo.name = $2
        "#,
		workspace_id as _,
		&repo_name
	)
	.fetch_all(&mut *database)
	.await
	.map_err(internal_server_error_response)?;

	let tags: Vec<String> = row.into_iter().map(|r| r.name).collect();

	let body = TagListBuilder::default()
		.name(repo_name)
		.tags(tags)
		.build()
		.map_err(internal_server_error_response)?;

	Ok((
		StatusCode::OK,
		[(
			HeaderName::from_static("Docker-Distribution-API-Version"),
			HeaderValue::from_static("registry/2.0"),
		)]
		.into_iter()
		.collect::<HeaderMap>(),
		Json(body),
	)
		.into_response())
}
