//! PUT manifest endpoint handler.
//!
//! This handler uploads a new manifest to the registry, validates it,
//! stores it in S3, and creates/updates tags as needed.

use std::{collections::BTreeMap, str::FromStr};

use aws_sdk_s3::primitives::ByteStream;
use axum::body::Body;
use headers::ContentType;
use models::{
	api::workspace::{deployment::*, runner::StreamRunnerDataForWorkspaceServerMsg},
	utils::{Base64String, StringifiedU16},
};
use oci_spec::image::{Digest, ImageConfiguration, ImageIndex, ImageManifest};
use rustis::commands::{GenericCommands, PubSubCommands};
use sha2::{Digest as _, Sha256};
use time::OffsetDateTime;

use crate::{models::permissions, redis::keys, routes::registry_patr_cloud::prelude::*};

macros::declare_registry_endpoint!(
	/// PUT manifest endpoint.
	///
	/// Uploads a new manifest to the registry. The manifest can be an OCI Image Manifest
	/// or an OCI Image Index (manifest list). All referenced blobs must already exist.
	PutManifest,
	PUT "/v2/{workspace_id}/{repo_name}/manifests/{reference}" {
		/// The workspace ID
		pub workspace_id: Uuid,
		/// The repository name
		#[preprocess(lowercase, regex = constants::REGISTRY_REPO_NAME_REGEX, length(max = 255))]
		pub repo_name: String,
		/// The manifest reference (tag name or digest)
		#[preprocess(regex = constants::REGISTRY_TAG_OR_DIGEST_REGEX)]
		pub reference: String,
	},
	request_headers = {
		/// The authorization header
		pub authorization: BearerToken,
		/// The content type of the request body
		pub content_type: ContentType,
	},
	response_headers = {
		/// Location of the uploaded manifest
		pub location: Location,
		/// The digest of the uploaded manifest
		pub docker_content_digest: DockerContentDigest,
		/// The docker distribution API version
		pub docker_distribution_api_version: DockerDistributionApiVersion,
	}
);

/// Handler for PUT /v2/{workspace_id}/{repo_name}/manifests/{reference}
///
/// This handler:
/// 1. Verifies user has push access to the repository
/// 2. Reads and computes the SHA256 digest of the manifest body
/// 3. Validates the digest against the reference (if reference is a digest)
/// 4. Stores the manifest in S3
/// 5. Records manifest metadata and repository linkage in the database
/// 6. If the manifest is a valid OCI ImageManifest, records config and layer
///    blobs
/// 7. Creates or updates a tag if the reference is a tag name
/// 8. Returns 201 Created with Location and Docker-Content-Digest headers
pub async fn upload_manifest(
	AuthenticatedRegistryAppRequest {
		request:
			RegistryProcessedApiRequest {
				path:
					PutManifestPathProcessed {
						workspace_id,
						repo_name,
						reference,
					},
				query: (),
				headers: PutManifestRequestHeaders {
					authorization: _,
					content_type,
				},
				body,
			},
		database,
		redis,
		s3,
		client_ip: _,
		user_data,
		config,
	}: AuthenticatedRegistryAppRequest<'_, PutManifestPath>,
) -> Result<RegistryResponse<PutManifestPath>, RegistryError> {
	trace!("PUT called on manifest");

	// Check that the user can push to this repository
	let repository_id = query!(
		r#"
		SELECT
			id AS "resource_id: Uuid"
		FROM
			container_registry_repository
		WHERE
			workspace_id = $1 AND
			name = $2 AND
			deleted IS NULL;
		"#,
		workspace_id as _,
		&repo_name,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or_else(|| {
		warn!("Repository `{workspace_id}/{repo_name}` not found");
		RegistryError::builder()
			.status(StatusCode::NOT_FOUND)
			.message("Repository not found")
			.code(ErrorCode::NameUnknown)
			.build()
	})
	.map(|row| row.resource_id)?;

	let permission_id = permissions::get_permission_id(
		&mut **database,
		Permission::ContainerRegistryRepository(ContainerRegistryRepositoryPermission::Push),
	)
	.await;

	let authorized =
		user_data.has_permission_on_resource(workspace_id, repository_id, permission_id);

	if !authorized {
		// Intentionally return a 404 to avoid leaking repository existence
		debug!("User not authorized to access repository");
		return RegistryError::builder()
			.status(StatusCode::NOT_FOUND)
			.message("Repository not found")
			.code(ErrorCode::NameUnknown)
			.build()
			.into_result();
	}

	// Read request body and compute digest
	let bytes = axum::body::to_bytes(body, constants::MAX_REGISTRY_MANIFEST_SIZE)
		.await
		.map_err(|e| {
			error!("Failed to read manifest body: {e}");

			RegistryError::builder()
				.status(StatusCode::BAD_REQUEST)
				.message("Failed to read manifest body")
				.code(ErrorCode::ManifestInvalid)
				.build()
		})?;
	let size = bytes.len();

	let reference_digest = Digest::from_str(&reference).ok();
	// Make sure the digest of the manifest matches the reference if the reference
	// is a digest. If the reference is a tag, we'll compute the digest and store it
	// under that tag.
	let digest = {
		let computed_digest = hex::encode(Sha256::digest(&bytes));

		debug!("Computed manifest digest: {computed_digest}");

		let digest_mismatch = reference_digest
			.as_ref()
			.map(|digest| digest.digest() != computed_digest)
			.unwrap_or(false);

		if digest_mismatch {
			warn!(
				"Manifest digest mismatch: reference `{reference}` does not match computed `{computed_digest}`"
			);
			return RegistryError::builder()
				.status(StatusCode::BAD_REQUEST)
				.message("Manifest digest does not match content")
				.detail(format!(
					"Provided reference: {reference}, Computed digest: {computed_digest}"
				))
				.code(ErrorCode::ManifestInvalid)
				.build()
				.into_result();
		}

		format!("sha256:{computed_digest}")
	};

	let content_type = content_type.to_string();
	let inserted_new = match content_type.as_str() {
		"application/vnd.oci.image.manifest.v1+json" |
		"application/vnd.docker.distribution.manifest.v2+json" => {
			// Process the manifest as an OCI Image Manifest.
			let Ok(manifest) = serde_json::from_slice::<ImageManifest>(&bytes) else {
				warn!("Failed to parse manifest as OCI Image Manifest");
				return RegistryError::builder()
					.status(StatusCode::BAD_REQUEST)
					.message("Failed to parse manifest as OCI Image Manifest")
					.code(ErrorCode::ManifestInvalid)
					.build()
					.into_result();
			};

			let config_digest = manifest.config().digest().to_string();
			let config_blob = serde_json::from_slice::<ImageConfiguration>(
				&s3.get_object()
					.bucket(&config.s3.bucket)
					.key(format!("registry/blobs/{config_digest}"))
					.send()
					.await?
					.body
					.collect()
					.await?
					.into_bytes(),
			)?;
			let platform = manifest
				.config()
				.platform()
				.as_ref()
				.map(|p| format!("{}/{}", p.os(), p.architecture()))
				.unwrap_or_else(|| format!("{}/{}", config_blob.os(), config_blob.architecture()));

			let inserted = query!(
				r#"
				INSERT INTO
					container_registry_manifest(
						digest,
						content_type,
						size,
						config_blob_digest,
						platform
					)
				VALUES
					(
						$1,
						$2,
						$3,
						$4,
						$5
					)
				ON CONFLICT (digest) DO NOTHING
				RETURNING digest;
				"#,
				digest,
				content_type,
				size as i32,
				config_digest,
				platform,
			)
			.fetch_optional(&mut **database)
			.await?
			.is_some();

			// Record each layer blob and clean up temporary Redis associations
			for layer in manifest.layers() {
				let blob_digest = layer.digest().to_string();

				query!(
					r#"
					INSERT INTO
						container_registry_manifest_blob(
							manifest_digest,
							blob_digest
						)
					VALUES
						($1, $2)
					ON CONFLICT (manifest_digest, blob_digest) DO NOTHING;
					"#,
					digest,
					&blob_digest
				)
				.execute(&mut **database)
				.await?;

				// The blob is now permanently linked via the manifest, so we can
				// remove the temporary Redis blob->repo association.
				let _ = redis
					.del(keys::repository_for_registry_blob(
						&repository_id,
						&blob_digest,
					))
					.await;
			}

			inserted
		}
		"application/vnd.oci.image.index.v1+json" |
		"application/vnd.docker.distribution.manifest.list.v2+json" => {
			// Process the manifest as an OCI Image Index (manifest list).
			let Ok(index) = serde_json::from_slice::<ImageIndex>(&bytes) else {
				warn!("Failed to parse manifest as OCI Image Index");
				return RegistryError::builder()
					.status(StatusCode::BAD_REQUEST)
					.message("Failed to parse manifest as OCI Image Index")
					.code(ErrorCode::ManifestInvalid)
					.build()
					.into_result();
			};

			let inserted = query!(
				r#"
				INSERT INTO
					container_registry_manifest(
						digest,
						content_type,
						size,
						config_blob_digest,
						platform
					)
				VALUES
					(
						$1,
						$2,
						$3,
						NULL,
						NULL
					)
				ON CONFLICT (digest) DO NOTHING
				RETURNING digest;
				"#,
				digest,
				content_type,
				size as i32,
			)
			.fetch_optional(&mut **database)
			.await?
			.is_some();

			// Record manifest references if this is an index
			for manifest in index.manifests() {
				let referenced_digest = manifest.digest().to_string();

				query!(
					r#"
					INSERT INTO
						container_registry_manifest_reference(
							digest,
							referenced_digest
						)
					VALUES
						($1, $2)
					ON CONFLICT (digest, referenced_digest) DO NOTHING;
					"#,
					digest,
					&referenced_digest
				)
				.execute(&mut **database)
				.await?;
			}

			inserted
		}
		_ => {
			warn!("Unsupported manifest content type: {content_type}");
			return RegistryError::builder()
				.status(StatusCode::UNSUPPORTED_MEDIA_TYPE)
				.message("Unsupported manifest content type")
				.detail(format!("Content-Type `{content_type}` is not supported"))
				.code(ErrorCode::ManifestInvalid)
				.build()
				.into_result();
		}
	};

	if inserted_new {
		// If the manifest was newly inserted, we need to upload it to S3. If it
		// already exists, we can skip the S3 upload since the content is
		// immutable and must be identical.
		s3.put_object()
			.bucket(&config.s3.bucket)
			.key(format!("registry/manifests/{digest}"))
			.content_type(content_type)
			.body(ByteStream::from(bytes))
			.send()
			.await
			.inspect_err(|e| {
				error!("Failed to upload manifest to S3: {e}");
			})?;
	}

	// Link this manifest to the repository
	query!(
		r#"
		INSERT INTO
			container_registry_repository_manifest(
				repository_id,
				manifest_digest,
				created_at
			)
		VALUES
			($1, $2, NOW())
		ON CONFLICT (repository_id, manifest_digest) DO NOTHING;
		"#,
		repository_id as _,
		digest
	)
	.execute(&mut **database)
	.await?;

	// Create or update tag if the reference is a tag name
	if reference_digest.is_none() {
		query!(
			r#"
			INSERT INTO
				container_registry_repository_tag(
					name,
					repository_id,
					manifest_digest,
					last_updated
				)
			VALUES
				($1, $2, $3, NOW())
			ON CONFLICT (repository_id, name)
			DO UPDATE SET
				manifest_digest = EXCLUDED.manifest_digest,
				last_updated = EXCLUDED.last_updated;
			"#,
			reference,
			repository_id as _,
			&digest
		)
		.execute(&mut **database)
		.await?;

		// Deploy-on-push: update all deployments using this repo+tag that
		// have opted in via deploy_on_push = true.
		if let Err(err) = auto_deploy_on_push(
			&mut *database,
			&mut *redis,
			repository_id,
			&reference,
			&digest,
			workspace_id,
		)
		.await
		{
			error!(
				"Failed to auto-deploy on push for repository {repository_id}, tag {reference}: {err}"
			);
		}
	}

	// Return 201 Created with location and digest headers
	RegistryResponse::builder()
		.status_code(StatusCode::CREATED)
		.headers(PutManifestResponseHeaders {
			location: Location::from_str(&format!(
				"/v2/{workspace_id}/{repo_name}/manifests/{digest}"
			))?,
			docker_content_digest: DockerContentDigest(digest),
			docker_distribution_api_version: DockerDistributionApiVersion,
		})
		.body(Body::empty())
		.build()
		.into_result()
}

/// Automatically update and redeploy all deployments that use the given
/// repository and tag with `deploy_on_push` enabled.
async fn auto_deploy_on_push(
	database: &mut DatabaseTransaction,
	redis: &mut rustis::client::Client,
	repository_id: Uuid,
	tag: &str,
	digest: &str,
	workspace_id: Uuid,
) -> Result<(), ErrorType> {
	let now = OffsetDateTime::now_utc();

	// Find all deployments using this repository + tag with deploy_on_push enabled
	let deployments = query!(
		r#"
		SELECT
			id AS "id: Uuid",
			name,
			registry,
			image_name,
			image_tag,
			runner AS "runner: Uuid",
			status AS "status: DeploymentStatus",
			repository_id AS "repository_id: Uuid",
			min_horizontal_scale,
			max_horizontal_scale,
			machine_type AS "machine_type: Uuid",
			deploy_on_push,
			startup_probe_port,
			startup_probe_path,
			startup_probe_port_type AS "startup_probe_port_type: Option<ExposedPortType>",
			liveness_probe_port,
			liveness_probe_path,
			liveness_probe_port_type AS "liveness_probe_port_type: Option<ExposedPortType>",
			current_live_digest
		FROM
			deployment
		WHERE
			repository_id = $1 AND
			image_tag = $2 AND
			deploy_on_push = TRUE AND
			deleted IS NULL;
		"#,
		repository_id as _,
		tag,
	)
	.fetch_all(&mut **database)
	.await?;

	for deployment in deployments {
		let deployment_id = deployment.id;
		let runner = deployment.runner;

		info!(
			"Auto-deploying deployment `{}` due to push of tag `{}` on repository `{}`",
			deployment_id, tag, repository_id
		);

		// Record the new digest in deployment history
		query!(
			r#"
			INSERT INTO
				deployment_deploy_history(
					deployment_id,
					image_digest,
					repository_id,
					created
				)
			VALUES
				($1, $2, $3, $4)
			ON CONFLICT
				(deployment_id, image_digest)
			DO NOTHING;
			"#,
			deployment_id as _,
			digest,
			repository_id as _,
			now as _,
		)
		.execute(&mut **database)
		.await?;

		// Update the deployment's live digest and set status to deploying
		query!(
			r#"
			UPDATE
				deployment
			SET
				current_live_digest = $1,
				status = $2
			WHERE
				id = $3;
			"#,
			digest,
			DeploymentStatus::Deploying as _,
			deployment_id as _,
		)
		.execute(&mut **database)
		.await?;

		// Fetch deployment details needed by the runner
		let ports = query!(
			r#"
			SELECT
				port,
				port_type AS "port_type: ExposedPortType"
			FROM
				deployment_exposed_port
			WHERE
				deployment_id = $1;
			"#,
			deployment_id as _
		)
		.fetch_all(&mut **database)
		.await?
		.into_iter()
		.map(|row| (StringifiedU16::new(row.port as u16), row.port_type))
		.collect::<BTreeMap<_, _>>();

		let environment_variables = query!(
			r#"
			SELECT
				name,
				value,
				secret_id AS "secret_id: Uuid"
			FROM
				deployment_environment_variable
			WHERE
				deployment_id = $1;
			"#,
			deployment_id as _
		)
		.fetch_all(&mut **database)
		.await?
		.into_iter()
		.filter_map(|env| {
			let value = match (env.value.clone(), env.secret_id.clone()) {
				(Some(val), None) => Some(EnvironmentVariableValue::String(val)),
				(None, Some(from_secret)) => Some(EnvironmentVariableValue::Secret { from_secret }),
				_ => {
					warn!(
						concat!(
							"corrupted deployment, cannot find environment variable value. ",
							"deployment_id: {}, env name: `{}`, value: {:?}`, secret_id: {:?}"
						),
						deployment_id, env.name, env.value, env.secret_id
					);
					None
				}
			};
			value.map(|v| (env.name, v))
		})
		.collect::<BTreeMap<_, _>>();

		let config_mounts = query!(
			r#"
			SELECT
				path,
				file
			FROM
				deployment_config_mounts
			WHERE
				deployment_id = $1;
			"#,
			deployment_id as _
		)
		.fetch_all(&mut **database)
		.await?
		.into_iter()
		.map(|row| (row.path, Base64String::from(row.file)))
		.collect::<BTreeMap<_, _>>();

		let volumes = query!(
			r#"
			SELECT
				volume_id AS "volume_id: Uuid",
				volume_mount_path
			FROM
				deployment_volume_mount
			WHERE
				deployment_id = $1;
			"#,
			deployment_id as _
		)
		.fetch_all(&mut **database)
		.await?
		.into_iter()
		.map(|row| (row.volume_id, row.volume_mount_path))
		.collect::<BTreeMap<_, _>>();

		let startup_probe = deployment
			.startup_probe_port
			.zip(deployment.startup_probe_path)
			.map(|(port, path)| DeploymentProbe {
				port: port as u16,
				path,
			});

		let liveness_probe = deployment
			.liveness_probe_port
			.zip(deployment.liveness_probe_path)
			.map(|(port, path)| DeploymentProbe {
				port: port as u16,
				path,
			});

		let registry = if deployment.registry == PatrRegistry.to_string() {
			DeploymentRegistry::PatrRegistry {
				registry: PatrRegistry,
				repository_id: deployment.repository_id.unwrap_or(repository_id),
			}
		} else {
			DeploymentRegistry::ExternalRegistry {
				registry: deployment.registry,
				image_name: deployment.image_name.unwrap_or_default(),
			}
		};

		// Notify the runner via Redis pubsub
		redis
			.publish(
				format!("{workspace_id}/runner/{runner}/stream"),
				serde_json::to_string(&StreamRunnerDataForWorkspaceServerMsg::DeploymentUpdated {
					deployment: WithId::new(
						deployment_id,
						Deployment {
							name: deployment.name,
							registry,
							image_tag: deployment.image_tag,
							runner,
							status: DeploymentStatus::Deploying,
							current_live_digest: Some(digest.to_string()),
							machine_type: deployment.machine_type,
						},
					),
					running_details: DeploymentRunningDetails {
						deploy_on_push: deployment.deploy_on_push,
						min_horizontal_scale: deployment.min_horizontal_scale as u16,
						max_horizontal_scale: deployment.max_horizontal_scale as u16,
						ports,
						environment_variables,
						startup_probe,
						liveness_probe,
						config_mounts,
						volumes,
					},
				})
				.map_err(|err| {
					ErrorType::server_error(format!("Failed to serialize deployment update: {err}"))
				})?,
			)
			.await?;

		info!("Successfully triggered auto-deploy for deployment `{deployment_id}`");
	}

	Ok(())
}
