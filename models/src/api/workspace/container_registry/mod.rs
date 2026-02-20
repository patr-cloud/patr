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

/// Contains manifest information of a repository
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ListableResource, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ContainerRepositoryManifestInfo {
	/// Manifest digest
	pub digest: String,
	/// The size of the manifest
	#[search(ty = "range")]
	pub size: u64,
	/// The platform of the manifest, if it's an image
	pub platform: String,
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
