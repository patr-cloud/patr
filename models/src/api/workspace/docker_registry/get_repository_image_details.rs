use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::{DockerRepositoryImageInfo, DockerRepositoryTagInfo};
use crate::{
	utils::{Paginated, Uuid},
	ApiRequest,
};

#[derive(
	Eq,
	Ord,
	Hash,
	Debug,
	Clone,
	Default,
	TypedPath,
	PartialEq,
	Serialize,
	PartialOrd,
	Deserialize,
)]
#[typed_path("/workspace/:workspace_id/docker-registry/:repository_id/image/:image_digest")]
pub struct GetDockerRepositoryImageDetailsPath {
	pub workspace_id: Uuid,
	pub repository_id: Uuid,
	pub image_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetDockerRepositoryImageDetailsRequest;

impl ApiRequest for GetDockerRepositoryImageDetailsRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetDockerRepositoryImageDetailsPath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = GetDockerRepositoryImageDetailsResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetDockerRepositoryImageDetailsResponse {
	#[serde(flatten)]
	pub image: DockerRepositoryImageInfo,
	pub tags: Vec<DockerRepositoryTagInfo>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{
		GetDockerRepositoryImageDetailsRequest,
		GetDockerRepositoryImageDetailsResponse,
	};
	use crate::{
		models::workspace::docker_registry::{
			DockerRepositoryImageInfo,
			DockerRepositoryTagInfo,
		},
		utils::DateTime,
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GetDockerRepositoryImageDetailsRequest,
			&[Token::UnitStruct {
				name: "GetDockerRepositoryImageDetailsRequest",
			}],
		);
	}

	#[test]
	fn assert_response_types_without_tags() {
		assert_tokens(
			&GetDockerRepositoryImageDetailsResponse {
				image: DockerRepositoryImageInfo {
					digest: "sha256:fea8895f450959fa676bcc1df0611ea93823a735a01205fd8622846041d0c7cf".to_string(),
					size: 9087654321,
					created: DateTime::default(),
				},
				tags: vec![],
			},
			&[
				Token::Map {
					len: None,
				},
				Token::Str("digest"),
				Token::Str("sha256:fea8895f450959fa676bcc1df0611ea93823a735a01205fd8622846041d0c7cf"),
				Token::Str("size"),
				Token::U64(9087654321),
				Token::Str("created"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("tags"),
				Token::Seq { len: Some(0) },
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_response_types_with_tags() {
		assert_tokens(
			&GetDockerRepositoryImageDetailsResponse {
				image: DockerRepositoryImageInfo {
					digest: "sha256:fea8895f450959fa676bcc1df0611ea93823a735a01205fd8622846041d0c7cf".to_string(),
					size: 9087654321,
					created: DateTime::default(),
				},
				tags: vec![
					DockerRepositoryTagInfo {
						tag: "stable".to_string(),
						last_updated: DateTime::default(),
					},
					DockerRepositoryTagInfo {
						tag: "beta".to_string(),
						last_updated: DateTime::default(),
					},
				],
			},
			&[
				Token::Map {
					len: None,
				},
				Token::Str("digest"),
				Token::Str("sha256:fea8895f450959fa676bcc1df0611ea93823a735a01205fd8622846041d0c7cf"),
				Token::Str("size"),
				Token::U64(9087654321),
				Token::Str("created"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("tags"),
				Token::Seq { len: Some(2) },
				Token::Struct {
					name: "DockerRepositoryTagInfo",
					len: 2,
				},
				Token::Str("tag"),
				Token::Str("stable"),
				Token::Str("lastUpdated"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::StructEnd,
				Token::Struct {
					name: "DockerRepositoryTagInfo",
					len: 2,
				},
				Token::Str("tag"),
				Token::Str("beta"),
				Token::Str("lastUpdated"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types_without_tags() {
		assert_tokens(
			&ApiResponse::success(GetDockerRepositoryImageDetailsResponse {
				image: DockerRepositoryImageInfo {
					digest: "sha256:fea8895f450959fa676bcc1df0611ea93823a735a01205fd8622846041d0c7cf".to_string(),
					size: 9087654321,
					created: DateTime::default(),
				},
				tags: vec![],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("digest"),
				Token::Str("sha256:fea8895f450959fa676bcc1df0611ea93823a735a01205fd8622846041d0c7cf"),
				Token::Str("size"),
				Token::U64(9087654321),
				Token::Str("created"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("tags"),
				Token::Seq { len: Some(0) },
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types_with_tags() {
		assert_tokens(
			&ApiResponse::success(GetDockerRepositoryImageDetailsResponse {
				image: DockerRepositoryImageInfo {
					digest: "sha256:fea8895f450959fa676bcc1df0611ea93823a735a01205fd8622846041d0c7cf".to_string(),
					size: 9087654321,
					created: DateTime::default(),
				},
				tags: vec![
					DockerRepositoryTagInfo {
						tag: "stable".to_string(),
						last_updated: DateTime::default(),
					},
					DockerRepositoryTagInfo {
						tag: "beta".to_string(),
						last_updated: DateTime::default(),
					},
				],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("digest"),
				Token::Str("sha256:fea8895f450959fa676bcc1df0611ea93823a735a01205fd8622846041d0c7cf"),
				Token::Str("size"),
				Token::U64(9087654321),
				Token::Str("created"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("tags"),
				Token::Seq { len: Some(2) },
				Token::Struct {
					name: "DockerRepositoryTagInfo",
					len: 2,
				},
				Token::Str("tag"),
				Token::Str("stable"),
				Token::Str("lastUpdated"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::StructEnd,
				Token::Struct {
					name: "DockerRepositoryTagInfo",
					len: 2,
				},
				Token::Str("tag"),
				Token::Str("beta"),
				Token::Str("lastUpdated"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
