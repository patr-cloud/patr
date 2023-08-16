use axum::{
	extract::Path,
	http::{HeaderMap, HeaderName, HeaderValue, StatusCode, Method},
	response::IntoResponse,
};
use models::utils::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathParams {
	workspace_id: Uuid,
	repo_name: String,
	digest: String,
}

#[axum::debug_handler]
pub(super) async fn handle(
	method: Method,
	Path(PathParams {
		workspace_id,
		repo_name,
		digest,
	}): Path<PathParams>,
) -> impl IntoResponse {
	
	(
		[(
			HeaderName::from_static("Docker-Distribution-API-Version"),
			HeaderValue::from_static("registry/2.0"),
		)]
		.into_iter()
		.collect::<HeaderMap>(),
		StatusCode::OK,
	)
}
