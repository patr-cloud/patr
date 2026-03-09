use aws_credential_types::Credentials;
use aws_sdk_s3::{
	Client as S3Client,
	config::{Builder as S3Builder, Region},
};
use axum::http::StatusCode;
use models::{api::workspace::container_registry::*, prelude::*};
use oci_spec::image::{Config, ImageManifest};

use crate::prelude::*;

pub async fn get_exposed_ports(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					GetContainerRepositoryExposedPortsPath {
						workspace_id: _,
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
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, GetContainerRepositoryExposedPortsRequest>,
) -> Result<AppResponse<GetContainerRepositoryExposedPortsRequest>, ErrorType> {
	info!("Starting: Get exposed ports");

	// Check if repository exists
	query!(
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
	let digest = query!(
		r#"
		SELECT
			manifest_digest AS "manifest_digest!"
		FROM
			(
				SELECT
					manifest_digest
				FROM
					container_registry_repository_tag
				WHERE
					repository_id = $1 AND
					name = $2

				UNION ALL

				SELECT
					manifest_digest
				FROM
					container_registry_repository_manifest
				WHERE
					repository_id = $1 AND
					manifest_digest = $2
			) AS combined;
		"#,
		repository_id as _,
		digest_or_tag as _
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::TagNotFound)?
	.manifest_digest;

	let s3 = S3Client::from_conf(
		S3Builder::new()
			.region(Region::new(state.config.s3.region.clone()))
			.endpoint_url(state.config.s3.endpoint.clone())
			.credentials_provider(
				Credentials::builder()
					.access_key_id(&state.config.s3.key)
					.secret_access_key(&state.config.s3.secret)
					.provider_name("Static")
					.build(),
			)
			.force_path_style(state.config.s3.force_path_style)
			.build(),
	);

	let manifest = s3
		.get_object()
		.bucket(&state.config.s3.bucket)
		.key(format!("manifests/{digest}"))
		.send()
		.await
		.map_err(|err| {
			error!("Failed to get manifest from S3: {}", err);
			ErrorType::InternalServerError
		})?
		.body
		.collect()
		.await
		.map_err(|err| {
			error!("Failed to read manifest body from S3: {}", err);
			ErrorType::InternalServerError
		})?;

	let Ok(manifest) = serde_json::from_slice::<ImageManifest>(&manifest.into_bytes()) else {
		error!("Failed to parse manifest JSON as an image manifest");
		return Err(ErrorType::InternalServerError);
	};

	let config_digest = manifest.config().digest().to_string();

	let config = s3
		.get_object()
		.bucket(&state.config.s3.bucket)
		.key(format!("blobs/{config_digest}"))
		.send()
		.await
		.map_err(|err| {
			error!("Failed to get config from S3: {}", err);
			ErrorType::InternalServerError
		})?
		.body
		.collect()
		.await
		.map_err(|err| {
			error!("Failed to read config body from S3: {}", err);
			ErrorType::InternalServerError
		})?;

	let Ok(config) = serde_json::from_slice::<Config>(&config.into_bytes()) else {
		error!("Failed to parse config JSON as an image config");
		return Err(ErrorType::InternalServerError);
	};

	let exposed_ports = config
		.exposed_ports()
		.clone()
		.unwrap_or_default()
		.into_iter()
		.filter_map(|port| port.parse::<u16>().ok())
		.collect();

	AppResponse::builder()
		.body(GetContainerRepositoryExposedPortsResponse {
			ports: exposed_ports,
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
