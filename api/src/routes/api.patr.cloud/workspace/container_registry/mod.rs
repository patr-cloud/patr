use axum::Router;
use jsonwebtoken::{Algorithm, Header};
use models::api::workspace::container_registry::*;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use sha2::{Digest, Sha256};
use time::{Duration, OffsetDateTime};

use crate::{prelude::*, utils::config::AppConfig};

mod create_repository;
mod delete_repository;
mod delete_repository_image;
mod get_repository_image_details;
mod get_repository_image_exposed_ports;
mod get_repository_info;
mod list_repositories;
mod list_repository_tags;

use self::{
	create_repository::*,
	delete_repository::*,
	delete_repository_image::*,
	get_repository_image_details::*,
	get_repository_image_exposed_ports::*,
	get_repository_info::*,
	list_repositories::*,
	list_repository_tags::*,
};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_auth_endpoint(create_repository, state)
		.mount_auth_endpoint(delete_repository, state)
		.mount_auth_endpoint(delete_repository_image, state)
		.mount_auth_endpoint(get_repository_image_details, state)
		.mount_auth_endpoint(get_repository_image_exposed_ports, state)
		.mount_auth_endpoint(get_repository_info, state)
		.mount_auth_endpoint(list_repositories, state)
		.mount_auth_endpoint(list_repository_tags, state)
		.with_state(state.clone())
}

async fn delete_docker_repository_image_in_registry(
	name: &str,
	username: &str,
	digest: &str,
	_config: &AppConfig,
) -> Result<(), ErrorType> {
	let iat = OffsetDateTime::now_utc();
	let response = reqwest::Client::new()
		.delete(format!(
			"{}://{}/v2/{}/manifests/{}",
			if r"config
				.container_registry
				.registry_url"
				.starts_with("localhost")
			{
				"http"
			} else {
				"https"
			},
			"config.container_registry.registry_url",
			name,
			digest
		))
		.bearer_auth(jsonwebtoken::encode(
			&Header {
				alg: Algorithm::ES256,
				kid: Some({
					let hash: Vec<u8> = Sha256::digest(
						/* config.container_registry.public_key.as_bytes() */ [],
					)
					.iter()
					.copied()
					.take(30)
					.collect();
					let encoded =
						base32::encode(base32::Alphabet::Rfc4648 { padding: false }, &hash);
					let mut kid = String::with_capacity(59);
					for (i, character) in encoded.chars().enumerate() {
						kid.push(character);
						if i % 4 == 3 && i != (encoded.len() - 1) {
							kid.push(':');
						}
					}
					kid
				}),
				..Default::default()
			},
			&RegistryToken {
				iss: "config.container_registry.issuer.clone()".to_string(),
				sub: username.to_string(),
				aud: "config.container_registry.service_name.clone()".to_string(),
				exp: iat + Duration::minutes(5), // 5 mins
				nbf: iat,
				iat,
				jti: thread_rng()
					.sample_iter(Alphanumeric)
					.map(char::from)
					.take(32)
					.collect(),
				access: vec![RegistryTokenAccess {
					r#type: "repository".to_string(),
					name: name.to_owned(),
					actions: vec!["delete".to_string()],
				}],
			},
			&jsonwebtoken::EncodingKey::from_ec_pem(
				// config.container_registry.private_key.as_bytes(),
				&[],
			)?,
		)?)
		.header(
			reqwest::header::ACCEPT,
			format!(
				"{}, {}",
				"application/vnd.docker.distribution.manifest.v2+json",
				"application/vnd.oci.image.manifest.v1+json"
			),
		)
		.send()
		.await?;

	// https://docs.docker.com/registry/spec/api/#delete-manifest
	// 200 => Accepted (Success)
	// 400 => Invalid Name or Reference
	// 404 => No Such Repository Error

	let response_status = response.status();
	if !response_status.is_success() && (response_status != 404 || response_status != 400) {
		return Err(ErrorType::server_error(
			"Failed to delete repository from registry",
		));
	}

	Ok(())
}
