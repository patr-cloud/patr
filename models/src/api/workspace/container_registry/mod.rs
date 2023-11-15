use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

mod create_repository;
// mod delete_repository;
mod delete_repository_image;
// mod get_exposed_port;
// mod get_repository_image_details;
// mod get_repository_info;
// mod get_repository_tag_details;
// mod list_repositories;

// mod list_repository_tags;

pub use self::{create_repository::*, delete_repository_image::*};

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
	pub last_updated: OffsetDateTime,
	/// The time the repository was created.
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
