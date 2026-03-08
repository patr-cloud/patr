use std::io::Write as _;

use api::routes::registry_patr_cloud::handlers::{blob::*, manifest::*};
use axum::body::Body;
use flate2::{Compression, write::GzEncoder};
use headers::{ContentLength, ContentType, HeaderMapExt as _};
use oci_spec::image::{
	Arch,
	ConfigBuilder,
	DescriptorBuilder,
	Digest,
	ImageConfigurationBuilder,
	ImageManifestBuilder,
	MediaType,
	Os,
	RootFsBuilder,
};
use sha2::{Digest as _, Sha256};

use crate::prelude::*;

/// Parse a content type string into a typed `ContentType`.
fn parse_content_type(s: &str) -> ContentType {
	let mut map = http::HeaderMap::new();
	map.insert(http::header::CONTENT_TYPE, s.parse().unwrap());
	map.typed_get().unwrap()
}

/// A minimal OCI image with config, layer, and manifest blobs and their
/// digests.
pub struct TestOciImage {
	pub config_bytes: Vec<u8>,
	pub config_digest: String,
	pub layer_bytes: Vec<u8>,
	pub layer_digest: String,
	pub manifest_bytes: Vec<u8>,
	pub manifest_digest: String,
}

/// Compute the `sha256:...` digest of the given bytes.
pub fn sha256_digest(data: &[u8]) -> String {
	let hash = Sha256::digest(data);
	format!("sha256:{}", hex::encode(hash))
}

/// Build a minimal valid OCI image (config + layer + manifest).
///
/// The `seed` parameter varies the layer content so that different seeds
/// produce images with different digests throughout.
pub fn build_minimal_oci_image(seed: u8) -> TestOciImage {
	// Build an uncompressed tar archive with a single file whose content
	// depends on `seed`.
	let file_content: Vec<u8> = vec![seed; 64];
	let mut tar_bytes = Vec::new();
	{
		let mut tar = tar::Builder::new(&mut tar_bytes);
		let mut header = tar::Header::new_gnu();
		header.set_size(file_content.len() as u64);
		header.set_mode(0o644);
		header.set_cksum();
		tar.append_data(&mut header, "data", &file_content[..])
			.unwrap();
		tar.finish().unwrap();
	}

	// diff_id is the digest of the *uncompressed* layer (per OCI spec)
	let diff_id = sha256_digest(&tar_bytes);

	// Gzip the tar to produce the actual layer blob
	let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
	encoder.write_all(&tar_bytes).unwrap();
	let layer_bytes = encoder.finish().unwrap();
	let layer_digest = sha256_digest(&layer_bytes);

	// Config: valid OCI image configuration
	let config = ImageConfigurationBuilder::default()
		.architecture(Arch::Amd64)
		.os(Os::Linux)
		.rootfs(
			RootFsBuilder::default()
				.typ("layers")
				.diff_ids(vec![diff_id])
				.build()
				.unwrap(),
		)
		.config(ConfigBuilder::default().build().unwrap())
		.build()
		.unwrap();
	let config_bytes = serde_json::to_vec(&config).unwrap();
	let config_digest = sha256_digest(&config_bytes);

	// Manifest: OCI image manifest v1
	let manifest = ImageManifestBuilder::default()
		.schema_version(2u32)
		.media_type(MediaType::ImageManifest)
		.config(
			DescriptorBuilder::default()
				.media_type(MediaType::ImageConfig)
				.digest(config_digest.parse::<Digest>().unwrap())
				.size(config_bytes.len() as u64)
				.build()
				.unwrap(),
		)
		.layers(vec![
			DescriptorBuilder::default()
				.media_type(MediaType::ImageLayerGzip)
				.digest(layer_digest.parse::<Digest>().unwrap())
				.size(layer_bytes.len() as u64)
				.build()
				.unwrap(),
		])
		.build()
		.unwrap();
	let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
	let manifest_digest = sha256_digest(&manifest_bytes);

	TestOciImage {
		config_bytes,
		config_digest,
		layer_bytes,
		layer_digest,
		manifest_bytes,
		manifest_digest,
	}
}

impl TestSetup {
	/// Initiate a chunked blob upload (POST without `?digest=`).
	/// Returns `(session_id, location)` parsed from the response headers.
	pub async fn initiate_chunked_upload(
		&self,
		api_token: &str,
		workspace_id: &Uuid,
		repo_name: &str,
	) -> (Uuid, String) {
		let response = self
			.make_registry_call(RegistryUnprocessedApiRequest::<InitiateBlobUploadPath> {
				path: InitiateBlobUploadPath {
					workspace_id: *workspace_id,
					repo_name: repo_name.to_string(),
				},
				query: InitiateBlobUploadQuery {
					mount: None,
					from: None,
					digest: None,
				},
				headers: InitiateBlobUploadRequestHeaders {
					authorization: BearerToken::from_str(api_token).unwrap(),
					content_length: ContentLength(0),
					content_type: OptionalHeader::new(None),
				},
				body: Body::empty(),
			})
			.await;

		assert_eq!(
			response.status_code(),
			StatusCode::ACCEPTED,
			"chunked upload initiation failed: {}",
			std::str::from_utf8(&response.as_bytes()).unwrap_or("<non-utf8>")
		);

		let location = response
			.maybe_header("location")
			.expect("expected Location header on chunked upload")
			.to_str()
			.unwrap()
			.to_string();

		let uuid_str = response
			.maybe_header("docker-upload-uuid")
			.expect("expected Docker-Upload-UUID header")
			.to_str()
			.unwrap()
			.to_string();

		let session_id =
			Uuid::parse_str(&uuid_str).expect("Docker-Upload-UUID is not a valid UUID");

		(session_id, location)
	}

	/// PATCH a blob chunk to an ongoing chunked upload session.
	pub async fn patch_blob_chunk(
		&self,
		api_token: &str,
		workspace_id: &Uuid,
		repo_name: &str,
		session_id: Uuid,
		data: &[u8],
	) -> axum_test::TestResponse {
		use api::routes::registry_patr_cloud::handlers::blob::UploadBlobChunkPath;

		self.make_registry_call(RegistryUnprocessedApiRequest::<UploadBlobChunkPath> {
			path: UploadBlobChunkPath {
				workspace_id: *workspace_id,
				repo_name: repo_name.to_string(),
				session_id,
			},
			query: (),
			headers:
				api::routes::registry_patr_cloud::handlers::blob::UploadBlobChunkRequestHeaders {
					authorization: BearerToken::from_str(api_token).unwrap(),
					content_type: ContentType::octet_stream(),
					content_length: OptionalHeader::new(Some(ContentLength(data.len() as u64))),
					content_range: OptionalHeader::new(None),
				},
			body: Body::from(data.to_vec()),
		})
		.await
	}

	/// Push a blob via monolithic upload (POST with `?digest=`).
	pub async fn push_blob(
		&self,
		api_token: &str,
		workspace_id: &Uuid,
		repo_name: &str,
		digest: &str,
		data: &[u8],
	) {
		let response = self
			.make_registry_call(RegistryUnprocessedApiRequest::<InitiateBlobUploadPath> {
				path: InitiateBlobUploadPath {
					workspace_id: *workspace_id,
					repo_name: repo_name.to_string(),
				},
				query: InitiateBlobUploadQuery {
					mount: None,
					from: None,
					digest: Some(digest.to_string()),
				},
				headers: InitiateBlobUploadRequestHeaders {
					authorization: BearerToken::from_str(api_token).unwrap(),
					content_length: ContentLength(data.len() as u64),
					content_type: OptionalHeader::new(Some(ContentType::octet_stream())),
				},
				body: Body::from(data.to_vec()),
			})
			.await;

		assert_eq!(
			response.status_code(),
			StatusCode::CREATED,
			"blob upload failed: {}",
			std::str::from_utf8(&response.into_bytes()).unwrap_or("<non-utf8>")
		);
	}

	/// Push a manifest (PUT with tag or digest reference).
	pub async fn push_manifest(
		&self,
		api_token: &str,
		workspace_id: &Uuid,
		repo_name: &str,
		reference: &str,
		manifest_bytes: &[u8],
	) -> axum_test::TestResponse {
		self.make_registry_call(RegistryUnprocessedApiRequest::<PutManifestPath> {
			path: PutManifestPath {
				workspace_id: *workspace_id,
				repo_name: repo_name.to_string(),
				reference: reference.to_string(),
			},
			query: (),
			headers: PutManifestRequestHeaders {
				authorization: BearerToken::from_str(api_token).unwrap(),
				content_type: parse_content_type("application/vnd.oci.image.manifest.v1+json"),
			},
			body: Body::from(manifest_bytes.to_vec()),
		})
		.await
	}

	/// Build, push, and return a complete test image. Pushes config blob, layer
	/// blob, then manifest with the given tag.
	pub async fn push_test_image(
		&self,
		api_token: &str,
		workspace_id: &Uuid,
		repo_name: &str,
		tag: &str,
	) -> TestOciImage {
		let image = build_minimal_oci_image(0);

		// Push config blob
		self.push_blob(
			api_token,
			workspace_id,
			repo_name,
			&image.config_digest,
			&image.config_bytes,
		)
		.await;

		// Push layer blob
		self.push_blob(
			api_token,
			workspace_id,
			repo_name,
			&image.layer_digest,
			&image.layer_bytes,
		)
		.await;

		// Push manifest
		let response = self
			.push_manifest(
				api_token,
				workspace_id,
				repo_name,
				tag,
				&image.manifest_bytes,
			)
			.await;

		assert_eq!(
			response.status_code(),
			StatusCode::CREATED,
			"manifest push failed: {}",
			std::str::from_utf8(&response.into_bytes()).unwrap_or("<non-utf8>")
		);

		image
	}
}
