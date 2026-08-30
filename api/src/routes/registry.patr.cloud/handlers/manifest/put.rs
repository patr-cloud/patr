//! PUT manifest endpoint handler.
//!
//! This handler uploads a new manifest to the registry, validates it,
//! stores it in S3, and creates/updates tags as needed.

use std::{collections::BTreeMap, str::FromStr};

use aws_sdk_s3::primitives::ByteStream;
use axum::body::Body;
use headers::ContentType;
use models::{
	api::workspace::{
		container_registry::ManifestKind,
		deployment::*,
		runner::StreamRunnerDataForWorkspaceServerMsg,
	},
	utils::{Base64String, StringifiedU16},
};
use oci_spec::image::{
	Descriptor,
	Digest,
	DigestAlgorithm,
	ImageConfiguration,
	ImageIndex,
	ImageManifest,
};
use rustis::commands::{GenericCommands, PubSubCommands};
use sha2::{Digest as _, Sha256, Sha512};
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
		#[cfg(feature = "cloud")]
		pub workspace_id: Uuid,
		/// The literal "registry" on self-hosted
		#[cfg(not(feature = "cloud"))]
		pub workspace_id: RegistryNamespace,
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
/// 6. If the manifest is a valid OCI ImageManifest, records config and layer blobs
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
	// Echo the client's path segment in the Location header (UUID on cloud,
	// "registry" on self-hosted) instead of the resolved workspace UUID.
	let registry_namespace = workspace_id;

	#[cfg(not(feature = "cloud"))]
	let workspace_id = {
		let _ = workspace_id;
		query!(
			r#"
			SELECT
				id AS "id: Uuid"
			FROM
				workspace
			WHERE
				deleted IS NULL
			LIMIT 1;
			"#
		)
		.fetch_one(&mut **database)
		.await?
		.id
	};

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
		database,
		Permission::ContainerRegistryRepository(ContainerRegistryRepositoryPermission::Push),
	)
	.await;

	let authorized =
		user_data.has_permission_on_resource(workspace_id, repository_id, permission_id);

	if !authorized {
		debug!("User lacks push access to repository");
		// Workspace members get a clear 403 (they can already list repos via the
		// API, so there's nothing to hide); non-members get a 404 so outsiders
		// can't enumerate private repositories.
		return if user_data.permissions.contains_key(&workspace_id) {
			RegistryError::builder()
				.status(StatusCode::FORBIDDEN)
				.message(format!(
					"You do not have push access to `{workspace_id}/{repo_name}`"
				))
				.code(ErrorCode::Denied)
				.build()
		} else {
			RegistryError::builder()
				.status(StatusCode::NOT_FOUND)
				.message("Repository not found")
				.code(ErrorCode::NameUnknown)
				.build()
		}
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

	// The digest algorithm comes from the reference when it's a digest, otherwise
	// sha256 for a tag push. Only sha256 and sha512 are supported — we never store
	// content we can't verify.
	let algorithm = reference_digest
		.as_ref()
		.map(|reference| reference.algorithm().clone())
		.unwrap_or(DigestAlgorithm::Sha256);
	let computed_hex = match &algorithm {
		DigestAlgorithm::Sha256 => hex::encode(Sha256::digest(&bytes)),
		DigestAlgorithm::Sha512 => hex::encode(Sha512::digest(&bytes)),
		other => {
			warn!("Unsupported manifest digest algorithm: {other}");
			return RegistryError::builder()
				.status(StatusCode::BAD_REQUEST)
				.message("Unsupported digest algorithm")
				.detail(format!("Digest algorithm `{other}` is not supported"))
				.code(ErrorCode::Unsupported)
				.build()
				.into_result();
		}
	};

	// If the reference is a digest, it must match the content.
	if reference_digest
		.as_ref()
		.is_some_and(|reference| reference.digest() != computed_hex)
	{
		warn!("Manifest digest mismatch: reference `{reference}` != computed `{computed_hex}`");
		return RegistryError::builder()
			.status(StatusCode::BAD_REQUEST)
			.message("Manifest digest does not match content")
			.code(ErrorCode::ManifestInvalid)
			.build()
			.into_result();
	}

	let digest = format!("{algorithm}:{computed_hex}");
	let media_type = content_type.to_string();

	// Classify the manifest. Any media type is accepted and stored as opaque
	// bytes; we enrich the relational tables best-effort per kind. Index media
	// types are unambiguous; an image and an artifact share the image-manifest
	// media type and are told apart by an `artifactType` / a non-image config.
	let is_index = matches!(
		media_type.as_str(),
		"application/vnd.oci.image.index.v1+json" |
			"application/vnd.docker.distribution.manifest.list.v2+json"
	);

	/// Locally-parsed manifest, boxed to keep the enum variants small.
	enum Parsed {
		Image(Box<ImageManifest>),
		Artifact(Box<ImageManifest>),
		Index(Box<ImageIndex>),
	}

	let parsed = if is_index {
		let Ok(index) = serde_json::from_slice::<ImageIndex>(&bytes) else {
			return RegistryError::builder()
				.status(StatusCode::BAD_REQUEST)
				.message("Failed to parse manifest as OCI Image Index")
				.code(ErrorCode::ManifestInvalid)
				.build()
				.into_result();
		};
		Parsed::Index(Box::new(index))
	} else {
		let Ok(manifest) = serde_json::from_slice::<ImageManifest>(&bytes) else {
			return RegistryError::builder()
				.status(StatusCode::BAD_REQUEST)
				.message("Failed to parse manifest as OCI Image Manifest")
				.code(ErrorCode::ManifestInvalid)
				.build()
				.into_result();
		};
		let config_media_type = manifest.config().media_type().to_string();
		let looks_like_image = manifest.artifact_type().is_none() &&
			matches!(
				config_media_type.as_str(),
				"application/vnd.oci.image.config.v1+json" |
					"application/vnd.docker.container.image.v1+json"
			);
		if looks_like_image {
			Parsed::Image(Box::new(manifest))
		} else {
			Parsed::Artifact(Box::new(manifest))
		}
	};

	let (kind, artifact_type, subject_digest) = match &parsed {
		Parsed::Image(_) => (ManifestKind::Image, None, None),
		Parsed::Artifact(manifest) => (
			ManifestKind::Artifact,
			manifest.artifact_type().as_ref().map(|ty| ty.to_string()),
			manifest
				.subject()
				.as_ref()
				.map(|sub| sub.digest().to_string()),
		),
		Parsed::Index(index) => (
			ManifestKind::Index,
			index.artifact_type().as_ref().map(|ty| ty.to_string()),
			index.subject().as_ref().map(|sub| sub.digest().to_string()),
		),
	};

	// Insert the base manifest row for any kind. The bytes in S3 are the source
	// of truth; the child tables below are derived metadata.
	let inserted_new = query!(
		r#"
		INSERT INTO
			container_registry_manifest(
				digest,
				media_type,
				size,
				kind,
				artifact_type,
				subject_digest
			)
		VALUES
			($1, $2, $3, $4, $5, $6)
		ON CONFLICT (digest) DO NOTHING
		RETURNING digest;
		"#,
		digest,
		media_type,
		size as i64,
		kind as _,
		artifact_type,
		subject_digest,
	)
	.fetch_optional(&mut **database)
	.await?
	.is_some();

	/// Whether a layer descriptor is a foreign / non-distributable layer — one
	/// whose blob lives at an external URL and is never stored in the registry.
	/// We reject these: a registry that can't reproduce an image it hosts is a
	/// footgun.
	fn is_foreign_layer(descriptor: &Descriptor) -> bool {
		let media_type = descriptor.media_type().to_string();
		media_type.contains("nondistributable") ||
			media_type.contains("foreign") ||
			descriptor
				.urls()
				.as_ref()
				.is_some_and(|urls| !urls.is_empty())
	}

	/// Translate a foreign-key violation from a child-table insert into the
	/// right OCI error. Clients push a manifest's blobs (and an index's
	/// children) before the manifest itself, so a violation means one wasn't
	/// pushed: a missing blob is `MANIFEST_BLOB_UNKNOWN`, a missing referenced
	/// manifest is `MANIFEST_UNKNOWN`. Anything else passes through unchanged.
	/// Letting the FK be the source of truth avoids a round-trip pre-check on
	/// every push.
	fn map_missing_reference(err: sqlx::Error) -> RegistryError {
		if let sqlx::Error::Database(db_err) = &err {
			if db_err.is_foreign_key_violation() {
				let constraint = db_err.constraint().unwrap_or_default();
				if constraint.contains("blob_digest") {
					warn!("Manifest references a blob not present in the registry");
					return RegistryError::builder()
						.status(StatusCode::NOT_FOUND)
						.message("Referenced blob is not present in the registry")
						.code(ErrorCode::ManifestBlobUnknown)
						.build();
				}
				if constraint.contains("referenced_digest") {
					warn!("Index references a child manifest not present in the registry");
					return RegistryError::builder()
						.status(StatusCode::NOT_FOUND)
						.message("Referenced manifest is not present in the registry")
						.code(ErrorCode::ManifestUnknown)
						.build();
				}
			}
		}
		RegistryError::from(err)
	}

	if inserted_new {
		match &parsed {
			Parsed::Image(manifest) => {
				// Reject foreign / non-distributable layers.
				if manifest.layers().iter().any(is_foreign_layer) {
					return RegistryError::builder()
						.status(StatusCode::BAD_REQUEST)
						.message("Non-distributable (foreign) layers are not supported")
						.code(ErrorCode::ManifestInvalid)
						.build()
						.into_result();
				}

				let config_digest = manifest.config().digest().to_string();

				// Read the config blob to recover the platform. Clients push blobs
				// before the manifest, so a missing config is MANIFEST_BLOB_UNKNOWN
				// rather than a 500 (layers are validated by their FK on insert).
				let config_object = s3
					.get_object()
					.bucket(&config.s3.bucket)
					.key(format!("registry/blobs/{config_digest}"))
					.send()
					.await
					.map_err(|err| {
						if err
							.raw_response()
							.map(|response| response.status().as_u16()) ==
							Some(404)
						{
							warn!("Manifest config blob `{config_digest}` is not present");
							RegistryError::builder()
								.status(StatusCode::NOT_FOUND)
								.message("Referenced blob is not present in the registry")
								.code(ErrorCode::ManifestBlobUnknown)
								.build()
						} else {
							RegistryError::from(err)
						}
					})?;
				let config_blob = serde_json::from_slice::<ImageConfiguration>(
					&config_object.body.collect().await?.into_bytes(),
				)?;
				let (os, architecture, variant, os_version) =
					match manifest.config().platform().as_ref() {
						Some(platform) => (
							platform.os().to_string(),
							platform.architecture().to_string(),
							platform.variant().clone(),
							platform.os_version().clone(),
						),
						None => (
							config_blob.os().to_string(),
							config_blob.architecture().to_string(),
							None,
							None,
						),
					};

				query!(
					r#"
					INSERT INTO
						container_registry_manifest_image(
							manifest_digest,
							config_blob_digest,
							os,
							architecture,
							variant,
							os_version
						)
					VALUES
						($1, $2, $3, $4, $5, $6)
					ON CONFLICT (manifest_digest) DO NOTHING;
					"#,
					digest,
					config_digest,
					os,
					architecture,
					variant,
					os_version,
				)
				.execute(&mut **database)
				.await
				.map_err(map_missing_reference)?;

				for (ordinal, layer) in manifest.layers().iter().enumerate() {
					let blob_digest = layer.digest().to_string();
					query!(
						r#"
						INSERT INTO
							container_registry_manifest_layer(
								manifest_digest,
								manifest_kind,
								ordinal,
								blob_digest,
								media_type,
								size
							)
						VALUES
							($1, $2, $3, $4, $5, $6)
						ON CONFLICT (manifest_digest, ordinal) DO NOTHING;
						"#,
						digest,
						ManifestKind::Image as _,
						ordinal as i32,
						blob_digest,
						layer.media_type().to_string(),
						layer.size() as i64,
					)
					.execute(&mut **database)
					.await
					.map_err(map_missing_reference)?;

					let _ = redis
						.del(keys::repository_for_registry_blob(
							&repository_id,
							&blob_digest,
						))
						.await;
				}
			}
			Parsed::Artifact(manifest) => {
				// An artifact's referenced blobs (its config + layers) are stored
				// as generic manifest layers so they remain pullable. `subject` is
				// deliberately not enforced (referrers may be pushed before the
				// subject exists).
				let config = manifest.config();
				let referenced_descriptors = std::iter::once(config).chain(manifest.layers());

				for (ordinal, descriptor) in referenced_descriptors.enumerate() {
					let blob_digest = descriptor.digest().to_string();
					query!(
						r#"
						INSERT INTO
							container_registry_manifest_layer(
								manifest_digest,
								manifest_kind,
								ordinal,
								blob_digest,
								media_type,
								size
							)
						VALUES
							($1, $2, $3, $4, $5, $6)
						ON CONFLICT (manifest_digest, ordinal) DO NOTHING;
						"#,
						digest,
						ManifestKind::Artifact as _,
						ordinal as i32,
						blob_digest,
						descriptor.media_type().to_string(),
						descriptor.size() as i64,
					)
					.execute(&mut **database)
					.await
					.map_err(map_missing_reference)?;

					let _ = redis
						.del(keys::repository_for_registry_blob(
							&repository_id,
							&blob_digest,
						))
						.await;
				}
			}
			Parsed::Index(index) => {
				// Children are pushed before the index; a missing one surfaces as
				// the referenced-manifest FK violation below (MANIFEST_UNKNOWN).
				for (ordinal, child) in index.manifests().iter().enumerate() {
					let (os, architecture, variant, os_version) = match child.platform().as_ref() {
						Some(platform) => (
							Some(platform.os().to_string()),
							Some(platform.architecture().to_string()),
							platform.variant().clone(),
							platform.os_version().clone(),
						),
						None => (None, None, None, None),
					};
					query!(
						r#"
						INSERT INTO
							container_registry_manifest_reference(
								manifest_digest,
								referenced_digest,
								ordinal,
								media_type,
								size,
								os,
								architecture,
								variant,
								os_version
							)
						VALUES
							($1, $2, $3, $4, $5, $6, $7, $8, $9)
						ON CONFLICT (manifest_digest, ordinal) DO NOTHING;
						"#,
						digest,
						child.digest().to_string(),
						ordinal as i32,
						child.media_type().to_string(),
						child.size() as i64,
						os,
						architecture,
						variant,
						os_version,
					)
					.execute(&mut **database)
					.await
					.map_err(map_missing_reference)?;
				}
			}
		}

		// Store the raw manifest bytes (immutable, so only on first insert).
		s3.put_object()
			.bucket(&config.s3.bucket)
			.key(format!("registry/manifests/{digest}"))
			.content_type(media_type.clone())
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
				"/v2/{registry_namespace}/{repo_name}/manifests/{digest}"
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
			status NOT IN ('stopped', 'errored') AND
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
			let value = match (env.value.clone(), env.secret_id) {
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
