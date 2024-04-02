use std::{collections::BTreeMap, str::FromStr};

use axum::{http::StatusCode, Router};
use models::{
	api::{
		workspace::{container_registry::*, infrastructure::deployment::ExposedPortType},
		WithId,
	},
	utils::{StringifiedU16, TotalCountHeader},
};
use time::OffsetDateTime;

use crate::{prelude::*, utils::config::AppConfig};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_auth_endpoint(create_container_repository, state)
		.mount_auth_endpoint(delete_container_repository_image, state)
		.mount_auth_endpoint(delete_container_repository, state)
		.mount_auth_endpoint(get_container_repository_exposed_ports, state)
		.mount_auth_endpoint(get_container_repository_image_details, state)
		.mount_auth_endpoint(get_container_repository_info, state)
		.mount_auth_endpoint(list_container_repositories, state)
		.mount_auth_endpoint(list_container_repository_tags, state)
		.with_state(state.clone())
}

async fn create_container_repository(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: CreateContainerRepositoryPath { workspace_id },
				query: _,
				headers,
				body: CreateContainerRepositoryRequestProcessed { name },
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, CreateContainerRepositoryRequest>,
) -> Result<AppResponse<CreateContainerRepositoryRequest>, ErrorType> {
	info!("Starting: Create container repository");

	// Check if repository already exist
	let already_exist = query!(
		r#"
		SELECT
			name
		FROM
			container_registry_repository
		WHERE
			name = $1
		AND
			workspace_id = $2 AND
			deleted IS NULL;
		"#,
		name as _,
		workspace_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.is_some();

	if already_exist {
		return Err(ErrorType::ResourceAlreadyExists);
	}

	// Create resource
	let resource_id = loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				*
			FROM
				resource
			WHERE
				id = $1;
			"#,
			uuid as _
		)
		.fetch_optional(&mut **database)
		.await?
		.is_some();

		if !exists {
			break uuid;
		}
	};

	query!(
		r#"
		INSERT INTO
			resource(
				id,
				resource_type_id,
				owner_id,
				created
			)
		VALUES
			(
				$1,
				(SELECT id FROM resource_type WHERE name = 'container_repository'),
				$2,
				NOW()
			);
		"#,
		resource_id as _,
		workspace_id as _,
	)
	.execute(&mut **database)
	.await?;

	// Create new repository in database
	query!(
		r#"
		INSERT INTO 
			container_registry_repository(
				id,
				workspace_id,
				name
			)
		VALUES
			($1, $2, $3);
		"#,
		resource_id as _,
		workspace_id as _,
		name as _
	)
	.execute(&mut **database)
	.await?;

	AppResponse::builder()
		.body(CreateContainerRepositoryResponse {
			id: WithId::new(resource_id, ()),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn delete_container_repository_image(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					DeleteContainerRepositoryImagePath {
						workspace_id,
						repository_id,
						digest,
					},
				query: _,
				headers,
				body,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, DeleteContainerRepositoryImageRequest>,
) -> Result<AppResponse<DeleteContainerRepositoryImageRequest>, ErrorType> {
	info!("Starting: Delete container repository image");

	// Get repository detail
	let repository_name = query!(
		r#"
		SELECT
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
	.map(|repo| repo.name)
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	let name = format!("{}/{}", workspace_id, repository_name);

	// Delete all tags for the given image
	let container_repo_tag_info: Vec<ContainerRepositoryTagInfo> = query!(
		r#"
		SELECT
			tag,
			last_updated
		FROM
			container_registry_repository_tag
		WHERE
			repository_id = $1 AND
			manifest_digest = $2;
		"#,
		repository_id as _,
		digest
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| ContainerRepositoryTagInfo {
		tag: row.tag,
		last_updated: row.last_updated.into(),
	})
	.collect();

	for tag in container_repo_tag_info {
		query!(
			r#"
			DELETE FROM
				container_registry_repository_tag
			WHERE
				repository_id = $1 AND
				tag = $2;
			"#,
			repository_id as _,
			tag.tag
		)
		.execute(&mut **database)
		.await?;
	}

	// Delete container repository image with digest from database
	query!(
		r#"
		DELETE FROM
			container_registry_repository_manifest
		WHERE
			repository_id = $1 AND
			manifest_digest = $2;
		"#,
		repository_id as _,
		digest
	)
	.execute(&mut **database)
	.await?;

	// Update storage used after deleting in usage history
	let total_storage = query!(
		r#"
		SELECT
			COALESCE(SUM(size), 0)::BIGINT as "size!"
		FROM
			container_registry_repository
		INNER JOIN
			container_registry_repository_manifest
		ON
			container_registry_repository.id 
			= container_registry_repository_manifest.repository_id
		INNER JOIN
			container_registry_manifest_blob
		ON
			container_registry_repository_manifest.manifest_digest 
			= container_registry_manifest_blob.manifest_digest
		INNER JOIN
			container_registry_repository_blob
		ON
			container_registry_manifest_blob.blob_digest 
			= container_registry_repository_blob.blob_digest
		WHERE
			container_registry_repository.workspace_id = $1;
		"#,
		workspace_id as _,
	)
	.fetch_one(&mut **database)
	.await
	.map(|repo| repo.size)?;

	todo!("Update usage history with the new size");

	// Delete container repository in registry
	todo!("Is god user's ID required or is current user ID okay?");

	delete_docker_repository_image_in_registry(&name, &user_data.username, &digest, &config)
		.await?;

	AppResponse::builder()
		.body(DeleteContainerRepositoryImageResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn delete_container_repository(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: DeleteContainerRepositoryPath {
					workspace_id,
					repository_id,
				},
				query: _,
				headers,
				body,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, DeleteContainerRepositoryRequest>,
) -> Result<AppResponse<DeleteContainerRepositoryRequest>, ErrorType> {
	info!("Starting: Delete container repository");

	// Check if any deployment currently running the repository
	let repo_being_used = query!(
		r#"
		SELECT
			id
		FROM
			deployment
		WHERE
			repository_id = $1 AND
			status != 'deleted';
		"#,
		repository_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.is_some();

	if repo_being_used {
		return Err(ErrorType::ResourceInUse);
	}

	// Delete from container registry
	let repository_name = query!(
		r#"
		SELECT
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
	.map(|repo| repo.name)
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	let name = format!("{}/{}", &workspace_id, repository_name);

	let images = query!(
		r#"
		SELECT
			manifest_digest
		FROM
			container_registry_repository_manifest
		WHERE
			repository_id = $1;
		"#,
		repository_id as _
	)
	.fetch_all(&mut **database)
	.await?;

	// Deleting all tags for the given repository
	query!(
		r#"
		DELETE FROM
			container_registry_repository_tag
		WHERE
			repository_id = $1;
		"#,
		repository_id as _
	)
	.execute(&mut **database)
	.await?;

	// Deleting all images for the given repository
	query!(
		r#"
		DELETE FROM
			container_registry_repository_manifest
		WHERE
			repository_id = $1;
		"#,
		repository_id as _
	)
	.execute(&mut **database)
	.await?;

	// Updating the name of docker repository to deleted
	query!(
		r#"
		UPDATE
			container_registry_repository
		SET
			deleted = $2
		WHERE
			id = $1;
		"#,
		repository_id as _,
		OffsetDateTime::now_utc()
	)
	.execute(&mut **database)
	.await?;

	for image in images {
		delete_docker_repository_image_in_registry(
			&name,
			&user_data.username,
			&image.manifest_digest,
			&config,
		)
		.await?;
	}

	AppResponse::builder()
		.body(DeleteContainerRepositoryResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn get_container_repository_exposed_ports(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					GetContainerRepositoryExposedPortsPath {
						workspace_id,
						repository_id,
						digest_or_tag,
					},
				query: _,
				headers,
				body,
			},
		database,
		redis: _,
		client_ip: _,
		config,
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

	let exposed_ports = reqwest::Client::new()
		.get(format!(
			"{}://{}/v2/{}/manifests/{}",
			if config
				.container_registry
				.registry_url
				.starts_with("localhost")
			{
				"http"
			} else {
				"https"
			},
			config.container_registry.registry_url,
			&repository_name,
			digest_or_tag
		))
		.bearer_auth(
			RegistryToken::new(
				config.container_registry.issuer.clone(),
				OffsetDateTime::now_utc(),
				user_data.username.clone(),
				config,
				vec![RegistryTokenAccess {
					r#type: "repository".to_string(),
					name: repository_name.to_string(),
					actions: vec!["pull".to_string()],
				}],
			)
			.to_string(
				config.container_registry.private_key.as_ref(),
				config.container_registry.public_key_der.as_ref(),
			)?,
		)
		.header(
			reqwest::header::CONTENT_TYPE,
			"application/vnd.docker.distribution.manifest.v1+prettyjws",
		)
		.send()
		.await?
		.json::<ContainerRepositoryManifest>()
		.await
		.map_err(|e| e)?
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

async fn get_container_repository_image_details(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					GetContainerRepositoryImageDetailsPath {
						workspace_id,
						repository_id,
						digest_or_tag,
					},
				query: _,
				headers,
				body,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, GetContainerRepositoryImageDetailsRequest>,
) -> Result<AppResponse<GetContainerRepositoryImageDetailsRequest>, ErrorType> {
	info!("Starting: Get image details");

	let (image_digest, image_created) = query!(
		r#"
		SELECT
			manifest_digest,
			created
		FROM
			container_registry_repository_manifest
		WHERE
			repository_id = $1 AND
			manifest_digest = $2;
		"#,
		repository_id as _,
		digest_or_tag
	)
	.fetch_optional(&mut **database)
	.await?
	.map(|image| (image.manifest_digest, image.created))
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	let image_tags = query!(
		r#"
		SELECT
			tag,
			last_updated
		FROM
			container_registry_repository_tag
		WHERE
			repository_id = $1 AND
			manifest_digest = $2;
		"#,
		repository_id as _,
		digest_or_tag
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| row.tag)
	.collect();

	let image_size = query!(
		r#"
		SELECT
			COALESCE(SUM(container_registry_repository_blob.size), 0)::BIGINT AS "image_size"
		FROM
			container_registry_manifest_blob
		INNER JOIN
			container_registry_repository_blob
		ON
			container_registry_manifest_blob.blob_digest
			= container_registry_repository_blob.blob_digest
		INNER JOIN
			container_registry_repository_manifest
		ON
			container_registry_manifest_blob.manifest_digest
			= container_registry_repository_manifest.manifest_digest
		WHERE
			container_registry_repository_manifest.repository_id = $1 AND
			container_registry_repository_manifest.manifest_digest = $2;
		"#,
		repository_id as _,
		digest_or_tag
	)
	.fetch_one(&mut **database)
	.await
	.map(|repo| repo.image_size)?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	AppResponse::builder()
		.body(GetContainerRepositoryImageDetailsResponse {
			digest: image_digest,
			size: image_size as u64,
			created: image_created,
			tags: image_tags,
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn get_container_repository_info(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetContainerRepositoryInfoPath {
					workspace_id,
					repository_id,
				},
				query: _,
				headers,
				body,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, GetContainerRepositoryInfoRequest>,
) -> Result<AppResponse<GetContainerRepositoryInfoRequest>, ErrorType> {
	info!("Starting: Get repository info");

	// Check if repository exist and get info
	let name = query!(
		r#"
		SELECT
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
	.map(|repo| repo.name)
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	let size = query!(
		r#"
		SELECT
			COALESCE(SUM(size), 0)::BIGINT as "size!"
		FROM
			container_registry_repository
		INNER JOIN
			container_registry_repository_manifest
		ON
			container_registry_repository.id 
			= container_registry_repository_manifest.repository_id
		INNER JOIN
			container_registry_manifest_blob
		ON
			container_registry_repository_manifest.manifest_digest 
			= container_registry_manifest_blob.manifest_digest
		INNER JOIN
			container_registry_repository_blob
		ON
			container_registry_manifest_blob.blob_digest 
			= container_registry_repository_blob.blob_digest
		WHERE
			container_registry_repository.workspace_id = $1;
		"#,
		workspace_id as _,
	)
	.fetch_one(&mut **database)
	.await
	.map(|repo| repo.size as u64)?;

	let last_updated = query!(
		r#"
		SELECT 
			GREATEST(
				resource.created, 
				(
					SELECT 
						COALESCE(created, TO_TIMESTAMP(0)) 
					FROM 
						container_registry_repository_manifest 
					WHERE 
						repository_id = $1
					ORDER BY
						created DESC
					LIMIT 1
				), 
				(
					SELECT 
						COALESCE(last_updated, TO_TIMESTAMP(0)) 
					FROM 
						container_registry_repository_tag 
					WHERE 
						repository_id = $1
					ORDER BY
						created DESC
					LIMIT 1
				)
			) as "last_updated!"
		FROM
			resource
		WHERE
			resource.id = $1;
		"#,
		repository_id as _
	)
	.fetch_one(&mut **database)
	.await
	.map(|row| row.last_updated)?;

	let created = query!(
		r#"
		SELECT
			MIN(created) AS created
		FROM
			container_registry_repository_manifest
		WHERE
			repository_id = $1;
		"#,
		repository_id as _
	)
	.fetch_one(&mut **database)
	.await
	.map(|repo| repo.created)?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	AppResponse::builder()
		.body(GetContainerRepositoryInfoResponse {
			repository: ContainerRepository {
				name,
				size,
				last_updated,
				created,
			},
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn list_container_repositories(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListContainerRepositoriesPath { workspace_id },
				query: _,
				headers,
				body,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, ListContainerRepositoriesRequest>,
) -> Result<AppResponse<ListContainerRepositoriesRequest>, ErrorType> {
	info!("Starting: List container repositories");

	let repositories = query!(
		r#" 
		SELECT
			container_registry_repository.id AS id,
			container_registry_repository.name AS name,
			COALESCE(SUM(container_registry_repository_blob.size), 0)::BIGINT AS size,
			MAX(container_registry_repository_tag.last_updated) AS last_updated,
			MIN(container_registry_repository_manifest.created) AS created
		FROM
			container_registry_repository
		LEFT JOIN
			container_registry_repository_manifest
		ON
			container_registry_repository.id 
			= container_registry_repository_manifest.repository_id
		LEFT JOIN
			container_registry_repository_tag
		ON
			container_registry_repository.id 
			= container_registry_repository_tag.repository_id
		LEFT JOIN
			container_registry_manifest_blob
		ON
			container_registry_repository_manifest.manifest_digest 
			= container_registry_manifest_blob.manifest_digest
		LEFT JOIN
			container_registry_repository_blob
		ON
			container_registry_manifest_blob.blob_digest 
			= container_registry_repository_blob.blob_digest
		WHERE
			container_registry_repository.workspace_id = $1
		GROUP BY
			container_registry_repository.id, container_registry_repository.name;
		"#,
		workspace_id as _
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|repo| {
		WithId::new(
			repo.id.into(),
			ContainerRepository {
				name: repo.name,
				size: repo.size.unwrap() as u64,
				last_updated: repo.last_updated.unwrap(),
				created: repo.created.unwrap(),
			},
		)
	})
	.collect();

	let total_count = query!(
		r#" 
		SELECT
			COUNT(*) AS count
		FROM
			container_registry_repository
		WHERE
			workspace_id = $1;
		"#,
		workspace_id as _
	)
	.fetch_one(&mut **database)
	.await
	.map(|repo| repo.count)?
	.ok_or(ErrorType::server_error(
		"Failed to get total repository count",
	))?;

	AppResponse::builder()
		.body(ListContainerRepositoriesResponse { repositories })
		.headers(ListContainerRepositoriesResponseHeaders {
			total_count: TotalCountHeader(total_count as usize),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn list_container_repository_tags(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListContainerRepositoryTagsPath {
					workspace_id,
					repository_id,
				},
				query: _,
				headers,
				body,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, ListContainerRepositoryTagsRequest>,
) -> Result<AppResponse<ListContainerRepositoryTagsRequest>, ErrorType> {
	info!("Starting: List repository tags");

	let tags = query!(
		r#"
		SELECT
			tag,
			manifest_digest,
			last_updated
		FROM
			container_registry_repository_tag
		WHERE
			repository_id = $1;
		"#,
		repository_id as _
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| ContainerRepositoryTagAndDigestInfo {
		tag: row.tag,
		last_updated: row.last_updated.into(),
		digest: row.manifest_digest,
	})
	.collect();

	let total_count = query!(
		r#" 
		SELECT
			COUNT(*) AS count
		FROM
			container_registry_repository_tag
		WHERE
			repository_id = $1;
		"#,
		repository_id as _
	)
	.fetch_one(&mut **database)
	.await
	.map(|repo| repo.count)?
	.ok_or(ErrorType::server_error(
		"Failed to get total repository count",
	))?;

	AppResponse::builder()
		.body(ListContainerRepositoryTagsResponse { tags })
		.headers(ListContainerRepositoryTagsResponseHeaders {
			total_count: TotalCountHeader(total_count as usize),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

pub async fn delete_docker_repository_image_in_registry(
	name: &str,
	username: &str,
	digest: &str,
	config: &AppConfig,
) -> Result<(), ErrorType> {
	let response = reqwest::Client::new()
		.delete(format!(
			"{}://{}/v2/{}/manifests/{}",
			if config
				.container_registry
				.registry_url
				.starts_with("localhost")
			{
				"http"
			} else {
				"https"
			},
			config.container_registry.registry_url,
			name,
			digest
		))
		.bearer_auth(
			RegistryToken::new(
				config.container_registry.issuer.clone(),
				OffsetDateTime::now_utc(),
				username.clone(),
				config,
				vec![RegistryTokenAccess {
					r#type: "repository".to_string(),
					name: name.to_owned(),
					actions: vec!["delete".to_string()],
				}],
			)
			.to_string(
				config.container_registry.private_key.as_ref(),
				config.container_registry.public_key_der.as_ref(),
			)?,
		)
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
	if !response_status.is_success() {
		let response_msg = response.text().await?;
		if response_status != 404 || response_status != 400 {
			return Err(ErrorType::server_error(
				"Failed to delete repository from registry",
			));
		};
	}

	Ok(())
}
