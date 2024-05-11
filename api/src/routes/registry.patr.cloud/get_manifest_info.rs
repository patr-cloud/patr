use axum::{
	extract::{Path, State},
	http::{header, HeaderValue, Method, StatusCode},
	response::IntoResponse,
	Json,
};
use monostate::MustBe;
use preprocess::Preprocessable;
use s3::Bucket;
use serde::{Deserialize, Serialize};

use super::{Error, ErrorItem, RegistryError};
use crate::prelude::*;

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
	/// The reference of the manifest
	#[preprocess(
		trim,
		lowercase,
		regex = r"$([a-f0-9]+)|([a-zA-Z0-9_][a-zA-Z0-9._-]{0,127})^"
	)]
	reference: String,
}

/// The response to a request to get information about a manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ManifestType {
	#[serde(rename_all = "camelCase")]
	/// The manifest is a single image manifest
	Manifest {
		/// The schema version of the manifest. Always 2
		schema_version: MustBe!(2u32),
		/// The media type of the manifest. Always
		/// application/vnd.oci.image.manifest.v1+json
		media_type: MustBe!("application/vnd.oci.image.manifest.v1+json"),
		/// The configuration object for the image
		config: ManifestConfig,
		/// The layers of the image
		layers: Vec<ManifestLayer>,
	},
	#[serde(rename_all = "camelCase")]
	/// The manifest is a multi-platform manifest
	Index {
		/// The schema version of the manifest. Always 2
		schema_version: MustBe!(2u32),
		/// The media type of the manifest. Always
		/// application/vnd.oci.image.index.v1+json
		media_type: MustBe!("application/vnd.oci.image.index.v1+json"),
		/// The manifests for different platforms
		manifests: Vec<PlatformManifest>,
	},
}

/// The configuration object for a manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestConfig {
	/// The media type of the configuration. Always
	/// application/vnd.oci.image.config.v1+json
	media_type: MustBe!("application/vnd.oci.image.config.v1+json"),
	/// The size of the configuration object
	size: u64,
	/// The digest of the configuration object
	digest: String,
}

/// A layer in a manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestLayer {
	/// The media type of the layer. Always
	/// application/vnd.oci.image.layer.v1.tar+gzip
	media_type: MustBe!("application/vnd.oci.image.layer.v1.tar+gzip"),
	/// The size of the layer
	size: u64,
	/// The digest of the layer
	digest: String,
}

/// A platform in a multi-platform manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformManifest {
	/// The digest of the manifest
	digest: String,
	/// The media type of the manifest. Always
	/// application/vnd.oci.image.manifest.v1+json
	media_type: MustBe!("application/vnd.oci.image.manifest.v1+json"),
	/// The platform information
	platform: PlatformInfo,
	/// The size of the manifest
	size: u64,
}

/// Information about a platform
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformInfo {
	/// The architecture of the platform
	architecture: String,
	/// The operating system of the platform
	os: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// The variant of the platform
	variant: Option<String>,
}

/*
curl -i https://registry.hub.docker.com/v2/library/ubuntu/manifests/latest -H 'Accept: application/vnd.oci.image.manifest.v1+json' -H 'Authorization: Bearer ...'
HTTP/1.1 200 OK
content-length: 1133
content-type: application/vnd.oci.image.index.v1+json
docker-content-digest: sha256:77906da86b60585ce12215807090eb327e7386c8fafb5402369e421f44eff17e
docker-distribution-api-version: registry/2.0
etag: "sha256:77906da86b60585ce12215807090eb327e7386c8fafb5402369e421f44eff17e"
date: Sat, 13 Apr 2024 07:39:41 GMT
strict-transport-security: max-age=31536000
ratelimit-limit: 100;w=21600
ratelimit-remaining: 100;w=21600
docker-ratelimit-source: 122.171.16.123

{
	"manifests": [
		{
			"digest": "sha256:aa772c98400ef833586d1d517d3e8de670f7e712bf581ce6053165081773259d",
			"mediaType": "application\/vnd.oci.image.manifest.v1+json",
			"platform": {
				"architecture": "amd64",
				"os": "linux"
			},
			"size": 424
		},
		{
			"digest": "sha256:7409efd2c351d36aaca162069e56a19fa2633944215cc478832a72d7eadfaf10",
			"mediaType": "application\/vnd.oci.image.manifest.v1+json",
			"platform": {
				"architecture": "arm",
				"os": "linux",
				"variant": "v7"
			},
			"size": 424
		},
		{
			"digest": "sha256:7185d738658e31c96b3ba0f9deaae1df46a5c405dc82025094a51e5e2072212a",
			"mediaType": "application\/vnd.oci.image.manifest.v1+json",
			"platform": {
				"architecture": "arm64",
				"os": "linux",
				"variant": "v8"
			},
			"size": 424
		},
		{
			"digest": "sha256:42c4c24813818c3794041522624f4e4def6ad49b900c3ac2762ffc61c25a4461",
			"mediaType": "application\/vnd.oci.image.manifest.v1+json",
			"platform": {
				"architecture": "ppc64le",
				"os": "linux"
			},
			"size": 424
		},
		{
			"digest": "sha256:c1c8dcac0e924911158b38838474ca79f06b16c281ab11bbac64db7421adf93c",
			"mediaType": "application\/vnd.oci.image.manifest.v1+json",
			"platform": {
				"architecture": "s390x",
				"os": "linux"
			},
			"size": 424
		}
	],
	"mediaType": "application\/vnd.oci.image.index.v1+json",
	"schemaVersion": 2
}



curl -i https://registry.hub.docker.com/v2/library/ubuntu/manifests/sha256:aa772c98400ef833586d1d517d3e8de670f7e712bf581ce6053165081773259d -H 'Accept: application/vnd.oci.image.manifest.v1+json' -H 'Authorization: Bearer ...'
HTTP/1.1 200 OK
content-length: 424
content-type: application/vnd.oci.image.manifest.v1+json
docker-content-digest: sha256:aa772c98400ef833586d1d517d3e8de670f7e712bf581ce6053165081773259d
docker-distribution-api-version: registry/2.0
etag: "sha256:aa772c98400ef833586d1d517d3e8de670f7e712bf581ce6053165081773259d"
date: Sat, 13 Apr 2024 07:41:30 GMT
strict-transport-security: max-age=31536000
ratelimit-limit: 100;w=21600
ratelimit-remaining: 99;w=21600
docker-ratelimit-source: 122.171.16.123

{
	"schemaVersion": 2,
	"mediaType": "application/vnd.oci.image.manifest.v1+json",
	"config": {
		"mediaType": "application/vnd.oci.image.config.v1+json",
		"size": 2297,
		"digest": "sha256:ca2b0f26964cf2e80ba3e084d5983dab293fdb87485dc6445f3f7bbfc89d7459"
	},
	"layers": [
		{
			"mediaType": "application/vnd.oci.image.layer.v1.tar+gzip",
			"size": 29538961,
			"digest": "sha256:bccd10f490ab0f3fba61b193d1b80af91b17ca9bdca9768a16ed05ce16552fcb"
		}
	]
}



curl -i https://registry.hub.docker.com/v2/library/ubuntu/blobs/sha256:ca2b0f26964cf2e80ba3e084d5983dab293fdb87485dc6445f3f7bbfc89d7459 -H 'Accept: application/vnd.oci.image.manifest.v1+json' -H 'Authorization: Bearer ...'
HTTP/2 200
date: Sat, 13 Apr 2024 07:44:48 GMT
content-type: application/octet-stream
content-length: 2297
cf-ray: 8739d53fdc571d24-BLR
cf-cache-status: HIT
accept-ranges: bytes
age: 881662
cache-control: public, max-age=14400
etag: "6c9314428349802af4bb76f646860c5e"
expires: Sat, 13 Apr 2024 11:44:48 GMT
last-modified: Tue, 27 Feb 2024 19:00:07 GMT
vary: Accept-Encoding
x-amz-id-2: oS8xxcNY7UJSS5URYWMIG3L8gQ9Rx0bBB1e1nINTCqD20H78x4AunCzytREW/JzGqDkDm53P8ls=
x-amz-request-id: 0XTESF66G61367FF
x-amz-server-side-encryption: AES256
x-amz-version-id: Qe5lfLoMcCHVSJB5eoQf8Lk1O.rN8ADm
server: cloudflare

{
	"architecture": "amd64",
	"config": {
		"Hostname": "",
		"Domainname": "",
		"User": "",
		"AttachStdin": false,
		"AttachStdout": false,
		"AttachStderr": false,
		"Tty": false,
		"OpenStdin": false,
		"StdinOnce": false,
		"Env": [
			"PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
		],
		"Cmd": [
			"/bin/bash"
		],
		"Image": "sha256:75bcf986b973ce28fbb229699d51b1dcf6b9fd1b5a4df91234430592574bd522",
		"Volumes": null,
		"WorkingDir": "",
		"Entrypoint": null,
		"OnBuild": null,
		"Labels": {
			"org.opencontainers.image.ref.name": "ubuntu",
			"org.opencontainers.image.version": "22.04"
		}
	},
	"container": "2c2eafcca730e58e38c77824394133063428cabe2968be90c3c99b909f4034f7",
	"container_config": {
		"Hostname": "2c2eafcca730",
		"Domainname": "",
		"User": "",
		"AttachStdin": false,
		"AttachStdout": false,
		"AttachStderr": false,
		"Tty": false,
		"OpenStdin": false,
		"StdinOnce": false,
		"Env": [
			"PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
		],
		"Cmd": [
			"/bin/sh",
			"-c",
			"#(nop) ",
			"CMD [\"/bin/bash\"]"
		],
		"Image": "sha256:75bcf986b973ce28fbb229699d51b1dcf6b9fd1b5a4df91234430592574bd522",
		"Volumes": null,
		"WorkingDir": "",
		"Entrypoint": null,
		"OnBuild": null,
		"Labels": {
			"org.opencontainers.image.ref.name": "ubuntu",
			"org.opencontainers.image.version": "22.04"
		}
	},
	"created": "2024-02-27T18:52:59.070788584Z",
	"docker_version": "24.0.5",
	"history": [
		{
			"created": "2024-02-27T18:52:57.011931013Z",
			"created_by": "/bin/sh -c #(nop)  ARG RELEASE",
			"empty_layer": true
		},
		{
			"created": "2024-02-27T18:52:57.033105379Z",
			"created_by": "/bin/sh -c #(nop)  ARG LAUNCHPAD_BUILD_ARCH",
			"empty_layer": true
		},
		{
			"created": "2024-02-27T18:52:57.056007352Z",
			"created_by": "/bin/sh -c #(nop)  LABEL org.opencontainers.image.ref.name=ubuntu",
			"empty_layer": true
		},
		{
			"created": "2024-02-27T18:52:57.070227736Z",
			"created_by": "/bin/sh -c #(nop)  LABEL org.opencontainers.image.version=22.04",
			"empty_layer": true
		},
		{
			"created": "2024-02-27T18:52:58.867932393Z",
			"created_by": "/bin/sh -c #(nop) ADD file:21c2e8d95909bec6f4acdaf4aed55b44ee13603681f93b152e423e3e6a4a207b in / "
		},
		{
			"created": "2024-02-27T18:52:59.070788584Z",
			"created_by": "/bin/sh -c #(nop)  CMD [\"/bin/bash\"]",
			"empty_layer": true
		}
	],
	"os": "linux",
	"rootfs": {
		"type": "layers",
		"diff_ids": [
			"sha256:5498e8c22f6996f25ef193ee58617d5b37e2a96decf22e72de13c3b34e147591"
		]
	}
}
*/

/// Handles the `GET /v2/{workspace_id}/{repo_name}/manifests/{reference}`
/// route.
#[axum::debug_handler]
pub(super) async fn handle(
	_method: Method,
	Path(path): Path<PathParams>,
	State(state): State<AppState>,
) -> Result<impl IntoResponse, Error> {
	let Ok(path) = path.preprocess() else {
		return Err(Error {
			errors: [ErrorItem {
				code: RegistryError::BlobUnknown,
				message: "Invalid repository name".to_string(),
				detail: "".to_string(),
			}],
			status_code: StatusCode::NOT_FOUND,
		});
	};

	let workspace_id = path.workspace_id;
	let mut database = state.database.begin().await?;

	// Check if the workspace exists
	let row = query!(
		r#"
		SELECT
			*
		FROM
			workspace
		WHERE
			id = $1 AND
			deleted IS NULL
		"#,
		workspace_id as _
	)
	.fetch_optional(&mut *database)
	.await?;

	let Some(_) = row else {
		return Err(Error {
			errors: [ErrorItem {
				code: RegistryError::BlobUnknown,
				message: "Invalid repository name".to_string(),
				detail: "".to_string(),
			}],
			status_code: StatusCode::NOT_FOUND,
		});
	};

	let manifests = query!(
		r#"
		SELECT
			container_registry_repository_manifest.*
		FROM
			container_registry_repository_manifest
		LEFT JOIN
			container_registry_repository_tag
		ON
			container_registry_repository_manifest.repository_id = container_registry_repository_tag.repository_id
		WHERE
			container_registry_repository_manifest.manifest_digest = $1 OR
			container_registry_repository_tag.tag = $1;
		"#,
		path.reference as _,
	)
	.fetch_all(&mut *database)
	.await?;

	let mut manifest_data = Vec::with_capacity(manifests.len());

	for manifest in manifests {
		let layers = query!(
			r#"
			WITH RECURSIVE blobs AS (
				SELECT
					root.blob_digest,
					root.parent_blob_digest
				FROM
					container_registry_manifest_blob AS root
				WHERE
					parent_blob_digest IS NULL AND
					manifest_digest = $1
				UNION ALL
				SELECT 
					container_registry_manifest_blob.blob_digest,
					container_registry_manifest_blob.parent_blob_digest
				FROM
					container_registry_manifest_blob
				JOIN
					blobs
				ON
					container_registry_manifest_blob.parent_blob_digest = blobs.blob_digest
			)
			SELECT
				container_registry_repository_blob.*
			FROM
				blobs
			INNER JOIN
				container_registry_repository_blob
			ON
				blobs.blob_digest = container_registry_repository_blob.blob_digest;
			"#,
			manifest.manifest_digest as _,
		)
		.fetch_all(&mut *database)
		.await?
		.into_iter()
		.map(|row| ManifestLayer {
			media_type: Default::default(),
			size: row.size as u64,
			digest: row.blob_digest,
		})
		.collect::<Vec<_>>();

		manifest_data.push((manifest, layers));
	}

	if let 0 = manifest_data.len() {
		return Err(Error {
			errors: [ErrorItem {
				code: RegistryError::BlobUnknown,
				message: "Repository not found".to_string(),
				detail: "".to_string(),
			}],
			status_code: StatusCode::NOT_FOUND,
		});
	}

	let bucket = Bucket::new(
		state.config.s3.bucket.as_str(),
		s3::Region::Custom {
			region: state.config.s3.region,
			endpoint: state.config.s3.endpoint,
		},
		{
			s3::creds::Credentials::new(
				Some(&state.config.s3.key),
				Some(&state.config.s3.secret),
				None,
				None,
				None,
			)?
		},
	)?;

	// Ehh. Is there a better way to do this?
	let (content_type, body) = if let (Some((manifest, layers)), true, true) = (
		manifest_data.first(),
		manifest_data.first().unwrap().0.manifest_digest == path.reference,
		manifest_data.len() == 1,
	) {
		(
			"application/vnd.oci.image.index.v1+json",
			ManifestType::Manifest {
				schema_version: Default::default(),
				media_type: Default::default(),
				config: ManifestConfig {
					media_type: Default::default(),
					size: {
						let s3_key = super::get_s3_object_name_for_blob(&manifest.manifest_digest);

						let (head, _) = bucket.head_object(&s3_key).await?;

						head.content_length.unwrap_or(2) as u64
					},
					digest: manifest.manifest_digest.clone(),
				},
				layers: layers.clone(),
			},
		)
	} else {
		(
			"application/vnd.oci.image.manifest.v1+json",
			ManifestType::Index {
				schema_version: Default::default(),
				media_type: Default::default(),
				manifests: {
					let mut manifests = vec![];
					for (manifest, layers) in manifest_data {
						manifests.push(PlatformManifest {
							digest: manifest.manifest_digest.clone(),
							media_type: Default::default(),
							platform: PlatformInfo {
								architecture: manifest.architecture,
								os: manifest.os,
								variant: Some(manifest.variant),
							},
							size: serde_json::to_string(&ManifestType::Manifest {
								schema_version: Default::default(),
								media_type: Default::default(),
								config: ManifestConfig {
									media_type: Default::default(),
									size: {
										let s3_key = super::get_s3_object_name_for_blob(
											&manifest.manifest_digest,
										);

										let (head, _) = bucket.head_object(&s3_key).await?;

										head.content_length.unwrap_or(2) as u64
									},
									digest: manifest.manifest_digest.clone(),
								},
								layers,
							})?
							.chars()
							.count() as u64,
						});
					}

					manifests
				},
			},
		)
	};

	Ok((
		StatusCode::OK,
		[(header::CONTENT_TYPE, HeaderValue::from_static(content_type))],
		Json(body),
	))
}
