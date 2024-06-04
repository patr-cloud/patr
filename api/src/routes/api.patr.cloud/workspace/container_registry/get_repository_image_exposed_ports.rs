use std::{collections::BTreeMap, str::FromStr};

use axum::http::StatusCode;
use jsonwebtoken::{Algorithm, Header};
use models::{
	api::workspace::{container_registry::*, infrastructure::deployment::ExposedPortType},
	prelude::*,
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use sha2::{Digest, Sha256};
use time::{Duration, OffsetDateTime};

use crate::prelude::*;

pub async fn get_repository_image_exposed_ports(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					GetContainerRepositoryExposedPortsPath {
						workspace_id,
						repository_id,
						digest_or_tag,
					},
				query: (),
				headers:
					GetContainerRepositoryExposedPortsRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: GetContainerRepositoryExposedPortsRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data,
	}: AuthenticatedAppRequest<'_, GetContainerRepositoryExposedPortsRequest>,
) -> Result<AppResponse<GetContainerRepositoryExposedPortsRequest>, ErrorType> {
	info!("Starting: Get exposed ports");

	// Check if repository exists
	let repository = query!(
		r#"
		SELECT
			id,
			name
		FROM
			container_registry_repository
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
		repository_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	// Check if tag exists
	query!(
		r#"
		SELECT
			tag,
			last_updated,
			manifest_digest
		FROM
			container_registry_repository_tag
		WHERE
			repository_id = $1 AND
			tag = $2;
		"#,
		repository_id as _,
		digest_or_tag
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::TagNotFound)?;

	let repository_name = format!("{}/{}", workspace_id, repository.name);
	let iat = OffsetDateTime::now_utc();

	let exposed_ports = reqwest::Client::new()
		.get(format!(
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
			&repository_name,
			digest_or_tag
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
				sub: user_data.username.to_string(),
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
					name: repository_name.to_string(),
					actions: vec!["pull".to_string()],
				}],
			},
			&jsonwebtoken::EncodingKey::from_ec_pem(
				// config.container_registry.private_key.as_bytes(),
				&[],
			)?,
		)?)
		.header(
			reqwest::header::CONTENT_TYPE,
			"application/vnd.docker.distribution.manifest.v1+prettyjws",
		)
		.send()
		.await?
		.json::<ContainerRepositoryManifest>()
		.await?
		.history
		.into_iter()
		.filter_map(|v1_comp_str| {
			serde_json::from_str::<V1Compatibility>(&v1_comp_str.v1_compatibility).ok()
		})
		.filter_map(|v1_comp| v1_comp.container_config.exposed_ports)
		.flat_map(IntoIterator::into_iter)
		.map(|(port, _)| port)
		.flat_map(|port| {
			if let Some((port, "tcp")) = port.split_once('/') {
				Some((StringifiedU16::from_str(port).ok()?, ExposedPortType::Http))
			} else {
				None
			}
		})
		.collect::<BTreeMap<_, _>>();

	AppResponse::builder()
		.body(GetContainerRepositoryExposedPortsResponse {
			ports: exposed_ports,
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
