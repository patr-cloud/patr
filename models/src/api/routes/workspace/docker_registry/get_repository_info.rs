use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::{DockerRepository, DockerRepositoryImageInfo};
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
#[typed_path("/workspace/:workspace_id/docker-registry/:repository_id")]
pub struct GetDockerRepositoryInfoPath {
	pub workspace_id: Uuid,
	pub repository_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetDockerRepositoryInfoRequest;

impl ApiRequest for GetDockerRepositoryInfoRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetDockerRepositoryInfoPath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = GetDockerRepositoryInfoResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetDockerRepositoryInfoResponse {
	#[serde(flatten)]
	pub repository: DockerRepository,
	pub images: Vec<DockerRepositoryImageInfo>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{
		GetDockerRepositoryInfoRequest,
		GetDockerRepositoryInfoResponse,
	};
	use crate::{
		models::workspace::docker_registry::{
			DockerRepository,
			DockerRepositoryImageInfo,
		},
		utils::{DateTime, Uuid},
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GetDockerRepositoryInfoRequest,
			&[Token::UnitStruct {
				name: "GetDockerRepositoryInfoRequest",
			}],
		);
	}

	#[test]
	fn assert_response_types_without_images() {
		assert_tokens(
			&GetDockerRepositoryInfoResponse {
				repository: DockerRepository {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					name: "test-repo".to_string(),
					size: 0,
					last_updated: DateTime::default(),
				},
				images: vec![],
			},
			&[
				Token::Map { len: None },
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("test-repo"),
				Token::Str("size"),
				Token::U64(0),
				Token::Str("lastUpdated"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("images"),
				Token::Seq { len: Some(0) },
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_response_types_with_images() {
		assert_tokens(
			&GetDockerRepositoryInfoResponse {
				repository: DockerRepository {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					name: "test-repo".to_string(),
					size: 1234567890,
					last_updated: DateTime::default(),
				},
				images: vec![
					DockerRepositoryImageInfo {
						digest: "sha256:fea8895f450959fa676bcc1df0611ea93823a735a01205fd8622846041d0c7cf".to_string(),
						size: 1234567890,
						created: DateTime::default(),
					},
					DockerRepositoryImageInfo {
						digest: "sha256:d89e1bee20d9cb344674e213b581f14fbd8e70274ecf9d10c514bab78a307845".to_string(),
						size: 5432167890,
						created: DateTime::default(),
					}
				],
			},
			&[
				Token::Map {
					len: None,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("test-repo"),
				Token::Str("size"),
				Token::U64(1234567890),
				Token::Str("lastUpdated"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("images"),
				Token::Seq {
					len: Some(2),
				},
				Token::Struct {
					name: "DockerRepositoryImageInfo",
					len: 3,
				},
				Token::Str("digest"),
				Token::Str("sha256:fea8895f450959fa676bcc1df0611ea93823a735a01205fd8622846041d0c7cf"),
				Token::Str("size"),
				Token::U64(1234567890),
				Token::Str("created"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::StructEnd,
				Token::Struct {
					name: "DockerRepositoryImageInfo",
					len: 3,
				},
				Token::Str("digest"),
				Token::Str("sha256:d89e1bee20d9cb344674e213b581f14fbd8e70274ecf9d10c514bab78a307845"),
				Token::Str("size"),
				Token::U64(5432167890),
				Token::Str("created"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types_without_images() {
		assert_tokens(
			&ApiResponse::success(GetDockerRepositoryInfoResponse {
				repository: DockerRepository {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					name: "test-repo".to_string(),
					size: 0,
					last_updated: DateTime::default(),
				},
				images: vec![],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("test-repo"),
				Token::Str("size"),
				Token::U64(0),
				Token::Str("lastUpdated"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("images"),
				Token::Seq { len: Some(0) },
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types_with_images() {
		assert_tokens(
			&ApiResponse::success(GetDockerRepositoryInfoResponse {
				repository: DockerRepository {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					name: "test-repo".to_string(),
					size: 1234567890,
					last_updated: DateTime::default(),
				},
				images: vec![
					DockerRepositoryImageInfo {
						digest: "sha256:fea8895f450959fa676bcc1df0611ea93823a735a01205fd8622846041d0c7cf".to_string(),
						size: 1234567890,
						created: DateTime::default(),
					},
					DockerRepositoryImageInfo {
						digest: "sha256:d89e1bee20d9cb344674e213b581f14fbd8e70274ecf9d10c514bab78a307845".to_string(),
						size: 5432167890,
						created: DateTime::default(),
					}
				],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("test-repo"),
				Token::Str("size"),
				Token::U64(1234567890),
				Token::Str("lastUpdated"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("images"),
				Token::Seq {
					len: Some(2),
				},
				Token::Struct {
					name: "DockerRepositoryImageInfo",
					len: 3,
				},
				Token::Str("digest"),
				Token::Str("sha256:fea8895f450959fa676bcc1df0611ea93823a735a01205fd8622846041d0c7cf"),
				Token::Str("size"),
				Token::U64(1234567890),
				Token::Str("created"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::StructEnd,
				Token::Struct {
					name: "DockerRepositoryImageInfo",
					len: 3,
				},
				Token::Str("digest"),
				Token::Str("sha256:d89e1bee20d9cb344674e213b581f14fbd8e70274ecf9d10c514bab78a307845"),
				Token::Str("size"),
				Token::U64(5432167890),
				Token::Str("created"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
