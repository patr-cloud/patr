use chrono::Utc;
use serde::{Deserialize, Serialize};

mod create_repository;
mod delete_repository;
mod delete_repository_image;
mod get_exposed_port;
mod get_repository_image_details;
mod get_repository_info;
mod get_repository_tag_details;
mod list_repositories;
mod list_repository_tags;

pub use self::{
	create_repository::*,
	delete_repository::*,
	delete_repository_image::*,
	get_exposed_port::*,
	get_repository_image_details::*,
	get_repository_info::*,
	get_repository_tag_details::*,
	list_repositories::*,
	list_repository_tags::*,
};
use crate::utils::{DateTime, Uuid};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DockerRepository {
	pub id: Uuid,
	pub name: String,
	pub size: u64,
	pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DockerRepositoryTagInfo {
	pub tag: String,
	pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DockerRepositoryImageInfo {
	pub digest: String,
	pub size: u64,
	pub created: DateTime<Utc>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{
		DockerRepository,
		DockerRepositoryImageInfo,
		DockerRepositoryTagInfo,
	};
	use crate::utils::{DateTime, Uuid};

	#[test]
	fn assert_docker_repository_types() {
		assert_tokens(
			&DockerRepository {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
				name: "test".to_string(),
				size: 1234567890,
				last_updated: DateTime::default(),
			},
			&[
				Token::Struct {
					name: "DockerRepository",
					len: 4,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("test"),
				Token::Str("size"),
				Token::U64(1234567890),
				Token::Str("lastUpdated"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_docker_repository_image_info_types() {
		assert_tokens(
			&DockerRepositoryImageInfo {
				digest: "sha256:fea8895f450959fa676bcc1df0611ea93823a735a01205fd8622846041d0c7cf".to_string(),
				size: 9087654321,
				created: DateTime::default(),
			},
			&[
				Token::Struct {
					name: "DockerRepositoryImageInfo",
					len: 3,
				},
				Token::Str("digest"),
				Token::Str("sha256:fea8895f450959fa676bcc1df0611ea93823a735a01205fd8622846041d0c7cf"),
				Token::Str("size"),
				Token::U64(9087654321),
				Token::Str("created"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::StructEnd,
			]
		);
	}

	#[test]
	fn assert_docker_repository_tag_info_types() {
		assert_tokens(
			&DockerRepositoryTagInfo {
				tag: "latest".to_string(),
				last_updated: DateTime::default(),
			},
			&[
				Token::Struct {
					name: "DockerRepositoryTagInfo",
					len: 2,
				},
				Token::Str("tag"),
				Token::Str("latest"),
				Token::Str("lastUpdated"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::StructEnd,
			],
		);
	}
}
