use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::prelude::*;

/// The endpoint to create a repository
mod create_repository;
/// The endpoint to delete a repository
mod delete_repository;
/// The endpoint to delete an image from a repository
mod delete_repository_manifest;
/// The endpoint to get the exposed ports of an image in a repository
mod get_exposed_ports;
/// The endpoint to get the registry-wide usage summary for a workspace
mod get_registry_usage;
/// The endpoint to get the details of a repository
mod get_repository_info;
/// The endpoint to get the details of an image in a repository
mod get_repository_manifest_details;
/// The endpoint to list all the repositories in a workspace
mod list_repositories;
/// The endpoint to list all the manifests of a repository
mod list_repository_manifests;
/// The endpoint to list all the tags of a repository
mod list_repository_tags;

pub use self::{
	create_repository::*,
	delete_repository::*,
	delete_repository_manifest::*,
	get_exposed_ports::*,
	get_registry_usage::*,
	get_repository_info::*,
	get_repository_manifest_details::*,
	list_repositories::*,
	list_repository_manifests::*,
	list_repository_tags::*,
};

/// Contains tag information of a repository
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ContainerRepositoryTagInfo {
	/// The tag
	pub tag: String,
	/// Last updated timestamp
	#[ts(type = "Date")]
	pub last_updated: OffsetDateTime,
}

/// Which of the three shapes a stored manifest takes. This is the single
/// definition shared between the database (as the `CONTAINER_REGISTRY_MANIFEST_KIND`
/// Postgres enum) and the API.
#[derive(
	Debug,
	Clone,
	Copy,
	PartialEq,
	Eq,
	Serialize,
	Deserialize,
	strum::EnumString,
	strum::Display,
	strum::VariantNames,
	ts_rs::TS,
)]
#[strum(serialize_all = "camelCase")]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
	not(target_arch = "wasm32"),
	derive(sqlx::Type),
	sqlx(type_name = "CONTAINER_REGISTRY_MANIFEST_KIND", rename_all = "lowercase")
)]
pub enum ManifestKind {
	/// A runnable single-platform image.
	Image,
	/// A multi-arch index (manifest list) that bundles per-platform images.
	Index,
	/// An OCI artifact (SBOM, signature, Helm chart, …) — not a runnable image.
	Artifact,
}

/// A single platform (OS + architecture) that an image runs on. An image has
/// one; a multi-arch index has one per child; an artifact has none.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct Platform {
	/// The operating system, e.g. `linux` or `windows`.
	pub os: String,
	/// The CPU architecture, e.g. `amd64` or `arm64`.
	pub architecture: String,
	/// The architecture variant, e.g. `v7` for `arm/v7`, if any.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub variant: Option<String>,
	/// The OS version, mainly used to distinguish Windows builds, if any.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub os_version: Option<String>,
}

/// One filesystem layer of an image: a stored blob that stacks with the others
/// to form the image's root filesystem. These are the "layers" a user sees when
/// they inspect an image.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ContainerRepositoryManifestLayer {
	/// The digest of the layer's blob.
	pub digest: String,
	/// The size of the layer, in bytes.
	pub size: u64,
	/// The media type of the layer.
	pub media_type: String,
}

/// Contains manifest information of a repository
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ListableResource, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ContainerRepositoryManifestInfo {
	/// Manifest digest
	pub digest: String,
	/// The size of the manifest
	#[search(ty = "range")]
	pub size: u64,
	/// Whether this manifest is an image, a multi-arch index, or an artifact.
	#[search(skip)]
	pub kind: ManifestKind,
	/// The platforms this manifest runs on: one for an image, many for an
	/// index, empty for an artifact.
	#[search(skip)]
	pub platforms: Vec<Platform>,
	/// The artifact type (media type) of the manifest, if it's an artifact.
	#[search(skip)]
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub artifact_type: Option<String>,
	/// The created timestamp
	#[ts(type = "Date")]
	pub created: OffsetDateTime,
	/// The tags that point to this manifest
	#[search(ty = "custom", name = "Vec<String>")]
	pub tags: Vec<String>,
}

/// Represents a repository of container images in Patr's in-build container
/// registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ListableResource, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ContainerRepository {
	/// The name of the repository.
	pub name: String,
	#[search(ty = "range")]
	/// The size of the repository in bytes.
	pub size: u64,
	/// The last time the repository was either created, updated or a tag was
	/// updated.
	///
	/// TODO: Change this to audit log
	#[ts(type = "Date")]
	pub last_updated: OffsetDateTime,
	/// The time the repository was created.
	///
	/// TODO: Change this to audit log
	#[ts(type = "Date")]
	pub created: OffsetDateTime,
}

#[cfg(test)]
mod test {
	use serde_test::{Configure, Token, assert_tokens};
	use time::OffsetDateTime;

	use super::ContainerRepository;

	#[test]
	fn assert_container_repository_types() {
		assert_tokens(
			&ContainerRepository {
				name: "test".to_string(),
				size: 1234567890,
				last_updated: OffsetDateTime::UNIX_EPOCH,
				created: OffsetDateTime::UNIX_EPOCH,
			}
			.readable(),
			&[
				Token::Struct {
					name: "ContainerRepository",
					len: 4,
				},
				Token::Str("name"),
				Token::Str("test"),
				Token::Str("size"),
				Token::U64(1234567890),
				Token::Str("lastUpdated"),
				Token::Str("1970-01-01 00:00:00.0 +00:00:00"),
				Token::Str("created"),
				Token::Str("1970-01-01 00:00:00.0 +00:00:00"),
				Token::StructEnd,
			],
		);
	}
}
