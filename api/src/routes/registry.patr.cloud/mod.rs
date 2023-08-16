use axum::{
	routing::{get, head, patch, post, put},
	Router,
};
use serde::{Deserialize, Serialize};

use crate::prelude::*;

mod get_blob_info;
mod get_registry_status;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RegistryError {
	BlobUnknown,
	BlobUploadInvalid,
	BlobUploadUnknown,
	DigestInvalid,
	ManifestBlobUnknown,
	ManifestInvalid,
	ManifestUnknown,
	NameInvalid,
	NameUnknown,
	SizeInvalid,
	Unauthorized,
	Denied,
	Unsupported,
	Toomanyrequests,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Error {
	pub errors: [ErrorItem; 1],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorItem {
	pub code: RegistryError,
	pub message: String,
	pub detail: String,
}

fn error(error: RegistryError, message: String) -> Error {
	Error {
		errors: [ErrorItem {
			code: error,
			message,
			detail: "".to_string(),
		}],
	}
}

pub fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.nest(
			"/v2",
			Router::new()
				.route("/", get(get_registry_status::handle))
				.route(
					"/:workspaceId/:repoName/blobs/:digest",
					head(get_blob_info::handle).get(get_blob_info::handle),
				), /*.route(
				   "/:workspaceId/:repoName/blobs/uploads/",
				   post(get_push_session_id),
			   ),  .route(
			   * 	"/:workspaceId/:repoName/blobs/:digest",
			   * 	head(get_blob_info).get(get_blob),
			   * )
			   * .route(
			   * 	"/:workspaceId/:repoName/manifests/:tag",
			   * 	put(add_manifest_to_repo),
			   * )
			   * .route(
			   * 	"/:workspaceId/:repoName/blobs/uploads/:digest",
			   * 	patch(patch_blob),
			   * ), */
		)
		.with_state(state.clone())
}

// pub async fn get_push_session_id(
// 	mut context: EveContext,
// 	_: NextHandler<EveContext, ErrorData>,
// ) -> Result<EveContext, Error> {
// 	let request = context.get_request();
// 	let client = reqwest::Client::builder()
// 		.redirect(reqwest::redirect::Policy::none())
// 		.build()?;

// 	let url = format!("http://localhost:5003{}", request.get_full_url());

// 	let mut request_builder = client
// 		.request(
// 			Method::try_from(request.get_method().to_string().as_str())?,
// 			url,
// 		)
// 		.body(request.get_body_bytes().to_vec());

// 	for (key, values) in context.get_request().get_headers().iter() {
// 		for value in values {
// 			if key != "host" {
// 				request_builder =
// 					request_builder.header(key.to_string(), value);
// 			}
// 		}
// 	}

// 	let response = request_builder.send().await?;

// 	let eve_response = context.get_response_mut();

// 	for (key, value) in response.headers().iter() {
// 		eve_response.set_header(key.as_str(), value.clone().to_str()?)
// 	}

// 	eve_response.set_status(response.status().as_u16());

// 	let buffered_response = response.bytes().await?.to_vec();
// 	eve_response.set_body_bytes(&buffered_response);

// 	Ok(context)
// }

// pub async fn get_blog_info(
// 	mut context: EveContext,
// 	_: NextHandler<EveContext, ErrorData>,
// ) -> Result<EveContext, Error> {
// 	let request = context.get_request();
// 	let client = reqwest::Client::builder()
// 		.redirect(reqwest::redirect::Policy::none())
// 		.build()?;

// 	let url = format!("http://localhost:5003{}", request.get_full_url());

// 	let mut request_builder = client
// 		.request(
// 			Method::try_from(request.get_method().to_string().as_str())?,
// 			url,
// 		)
// 		.body(request.get_body_bytes().to_vec());

// 	for (key, values) in context.get_request().get_headers().iter() {
// 		for value in values {
// 			if key != "host" {
// 				request_builder =
// 					request_builder.header(key.to_string(), value);
// 			}
// 		}
// 	}

// 	let response = request_builder.send().await?;

// 	let eve_response = context.get_response_mut();

// 	for (key, value) in response.headers().iter() {
// 		eve_response.set_header(key.as_str(), value.clone().to_str()?)
// 	}

// 	eve_response.set_status(response.status().as_u16());

// 	let buffered_response = response.bytes().await?.to_vec();
// 	eve_response.set_body_bytes(&buffered_response);

// 	Ok(context)
// }
// pub async fn add_manifest_to_repo(
// 	mut context: EveContext,
// 	_: NextHandler<EveContext, ErrorData>,
// ) -> Result<EveContext, Error> {
// 	// Can use unwrap here as, docker cli append the latest tag while pushing if
// 	// no tag is provided

// 	let workspace_id =
// 		Uuid::parse_str(context.get_param("workspaceId").unwrap()).unwrap();
// 	let repo_name = context.get_param(request_keys::REPO_NAME).unwrap().clone();
// 	let tag = context.get_param(request_keys::TAG).unwrap().clone();

// 	let request = context.get_request();
// 	let client = reqwest::Client::builder()
// 		.redirect(reqwest::redirect::Policy::none())
// 		.build()?;

// 	let url = format!("http://localhost:5003{}", request.get_full_url());

// 	let body = request.get_body()?;
// 	#[derive(Debug, Clone, Serialize, Deserialize)]
// 	#[serde(rename_all = "camelCase")]
// 	pub struct ImageManifest {
// 		pub layers: Vec<Layers>,
// 	}

// 	#[derive(Debug, Clone, Serialize, Deserialize)]
// 	#[serde(rename_all = "camelCase")]
// 	pub struct Layers {
// 		pub media_type: String,
// 		pub size: Option<usize>,
// 		pub digest: String,
// 	}

// 	let manifest = serde_json::from_str::<ImageManifest>(&body)?;
// 	let mut manifest_size = 0;
// 	for layer in &manifest.layers {
// 		if let Some(size) = layer.size {
// 			manifest_size += size;
// 		} else {
// 			// Make a head request and get the content-length header
// 			let client = reqwest::Client::new();
// 			let response = client
// 				.head(format!(
// 					"http://localhost:5003/v2/{}/{}/blobs/{}/",
// 					workspace_id, repo_name, layer.digest
// 				))
// 				.send()
// 				.await?;
// 			let header_content_length =
// 				response.headers().get("content-length").unwrap();
// 			let header_content_length: usize =
// 				header_content_length.to_str()?.parse()?;
// 			manifest_size += header_content_length
// 		}
// 	}

// 	let mut request_builder = client
// 		.request(
// 			Method::try_from(request.get_method().to_string().as_str())?,
// 			url,
// 		)
// 		.body(request.get_body_bytes().to_vec());

// 	for (key, values) in context.get_request().get_headers().iter() {
// 		for value in values {
// 			if key != "host" {
// 				request_builder =
// 					request_builder.header(key.to_string(), value);
// 			}
// 		}
// 	}

// 	let repository = db::get_docker_repository_by_name(
// 		context.get_database_connection(),
// 		&repo_name,
// 		&workspace_id,
// 	)
// 	.await?
// 	.status(404)
// 	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

// 	if service::docker_repo_storage_limit_crossed(
// 		context.get_database_connection(),
// 		&workspace_id,
// 		manifest_size,
// 	)
// 	.await?
// 	{
// 		return Error::as_result()
// 			.status(400)
// 			.body(error!(REPOSITORY_SIZE_LIMIT_EXCEEDED).to_string())?;
// 	}

// 	// Get total repo size for workspace

// 	let response = request_builder.send().await?;

// 	let image_digest = response.headers().get("docker-content-digest").unwrap();

// 	db::create_docker_repository_digest(
// 		context.get_database_connection(),
// 		&repository.id,
// 		image_digest.to_str()?,
// 		manifest_size as u64,
// 		&Utc::now(),
// 	)
// 	.await?;

// 	let total_storage =
// 		db::get_total_size_of_docker_repositories_for_workspace(
// 			context.get_database_connection(),
// 			&workspace_id,
// 		)
// 		.await?;

// 	db::update_docker_repo_usage_history(
// 		context.get_database_connection(),
// 		&workspace_id,
// 		&(((total_storage as f64) / (1000f64 * 1000f64 * 1000f64)).ceil()
// 			as i64),
// 		&Utc::now(),
// 	)
// 	.await?;

// 	db::set_docker_repository_tag_details(
// 		context.get_database_connection(),
// 		&repository.id,
// 		tag.as_str(),
// 		image_digest.to_str()?,
// 		&Utc::now(),
// 	)
// 	.await?;

// 	// TODO - Update deployment if there are any deployment for this repository

// 	let eve_response = context.get_response_mut();

// 	for (key, value) in response.headers().iter() {
// 		eve_response.set_header(key.as_str(), value.clone().to_str()?)
// 	}

// 	eve_response.set_status(response.status().as_u16());

// 	let buffered_response = response.bytes().await?.to_vec();

// 	eve_response.set_body_bytes(&buffered_response);

// 	Ok(context)
// }

// pub async fn patch_blog(
// 	mut context: EveContext,
// 	_: NextHandler<EveContext, ErrorData>,
// ) -> Result<EveContext, Error> {
// 	let request = context.get_request();
// 	let client = reqwest::Client::builder()
// 		.redirect(reqwest::redirect::Policy::none())
// 		.build()?;

// 	let url = format!("http://localhost:5003{}", request.get_full_url());
// 	let mut request_builder = client
// 		.request(
// 			Method::try_from(request.get_method().to_string().as_str())?,
// 			url,
// 		)
// 		.body(request.get_body_bytes().to_vec());

// 	for (key, values) in context.get_request().get_headers().iter() {
// 		for value in values {
// 			if key != "host" {
// 				request_builder =
// 					request_builder.header(key.to_string(), value);
// 			}
// 		}
// 	}

// 	let response = request_builder.send().await?;

// 	let eve_response = context.get_response_mut();

// 	for (key, value) in response.headers().iter() {
// 		eve_response.set_header(key.as_str(), value.clone().to_str()?)
// 	}

// 	eve_response.set_status(response.status().as_u16());

// 	let buffered_response = response.bytes().await?.to_vec();
// 	eve_response.set_body_bytes(&buffered_response);

// 	Ok(context)
// }
