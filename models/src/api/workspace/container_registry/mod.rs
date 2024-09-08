use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// The endpoint to create a repository
mod create_repository;
/// The endpoint to delete a repository
mod delete_repository;
/// The endpoint to delete an image from a repository
mod delete_repository_image;
/// The endpoint to get the exposed ports of an image in a repository
mod get_exposed_ports;
/// The endpoint to get the details of an image in a repository
mod get_repository_image_details;
/// The endpoint to get the details of a repository
mod get_repository_info;
/// The endpoint to list all the repositories in a workspace
mod list_repositories;
/// The endpoint to list all the tags of a repository
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
