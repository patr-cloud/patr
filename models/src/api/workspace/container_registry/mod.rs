use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;

mod create_repository;
mod delete_repository;
mod delete_repository_image;
mod get_exposed_ports;
mod get_repository_image_details;
mod get_repository_info;
mod list_repositories;
mod list_repository_tags;

pub use self::{
	create_repository::*,
	delete_repository::*,
	delete_repository_image::*,
	get_exposed_ports::*,
	get_repository_image_details::*,
	get_repository_info::*,
	list_repositories::*,
	list_repository_tags::*,
};
/// Contains tag information of a repository
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContainerRepositoryTagInfo {
	/// The tag
	pub tag: String,
	/// Last updated timestamp
	pub last_updated: OffsetDateTime,
}
/// Contains image information of a repository
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContainerRepositoryImageInfo {
	/// Image digest
	pub digest: String,
	/// The size of the image
	pub size: u64,
	/// The created timestamp
	pub created: OffsetDateTime,
}
/// Represents a repository of container images in Patr's in-build container
/// registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContainerRepository {
	/// The name of the repository.
	pub name: String,
	/// The size of the repository in bytes.
	pub size: u64,
	/// The last time the repository was either created, updated or a tag was
	/// updated.
	///
	/// TODO: Change this to audit log
	pub last_updated: OffsetDateTime,
	/// The time the repository was created.nlas
	///
	/// TODO: Change this to audit log
	pub created: OffsetDateTime,
}

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct V1Compatibility {
	pub container_config: DockerRepositoryExposedPort,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct DockerRepositoryExposedPort {
	pub exposed_ports: Option<HashMap<String, Value>>,
}

/// Container reposiotry manifest which will be used to
/// parse the json response from the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerRepositoryManifest {
	/// The history
	pub history: Vec<V1CompatibilityHolder>,
}

#[derive(Serialize, Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct V1CompatibilityHolder {
	pub v1_compatibility: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryToken {
	pub iss: String,
	pub sub: String,
	pub aud: String,
	pub exp: OffsetDateTime,
	pub nbf: OffsetDateTime,
	pub iat: OffsetDateTime,
	pub jti: String,
	pub access: Vec<RegistryTokenAccess>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryTokenAccess {
	pub r#type: String,
	pub name: String,
	pub actions: Vec<String>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Configure, Token};
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
