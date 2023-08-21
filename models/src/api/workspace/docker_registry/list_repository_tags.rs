use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::DockerRepositoryTagInfo;
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
#[typed_path("/workspace/:workspace_id/docker-registry/:repository_id/tag")]
pub struct ListDockerRepositoryTagsPath {
	pub workspace_id: Uuid,
	pub repository_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListDockerRepositoryTagsRequest;

impl ApiRequest for ListDockerRepositoryTagsRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListDockerRepositoryTagsPath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = ListDockerRepositoryTagsResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListDockerRepositoryTagsResponse {
	pub tags: Vec<DockerRepositoryTagAndDigestInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DockerRepositoryTagAndDigestInfo {
	#[serde(flatten)]
	pub tag_info: DockerRepositoryTagInfo,
	pub digest: String,
}

#[cfg(test)]
mod test {
	use chrono::{TimeZone, Utc};
	use serde_test::{assert_tokens, Token};

	use super::{
		DockerRepositoryTagAndDigestInfo,
		ListDockerRepositoryTagsRequest,
		ListDockerRepositoryTagsResponse,
	};
	use crate::{
		models::workspace::docker_registry::DockerRepositoryTagInfo,
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ListDockerRepositoryTagsRequest,
			&[Token::UnitStruct {
				name: "ListDockerRepositoryTagsRequest",
			}],
		);
	}

	#[test]
	fn assert_response_types_without_tags() {
		assert_tokens(
			&ListDockerRepositoryTagsResponse { tags: vec![] },
			&[
				Token::Struct {
					name: "ListDockerRepositoryTagsResponse",
					len: 1,
				},
				Token::Str("tags"),
				Token::Seq { len: Some(0) },
				Token::SeqEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types_with_tags() {
		assert_tokens(
			&ListDockerRepositoryTagsResponse {
				tags: vec![
					DockerRepositoryTagAndDigestInfo {
                        tag_info: DockerRepositoryTagInfo {
                            tag: "stable".to_string(),
                            last_updated: Utc.timestamp_opt(0, 0).unwrap().into(),
                        },
                        digest: "sha256:fea8895f450959fa676bcc1df0611ea93823a735a01205fd8622846041d0c7cf".to_string(),
                    },
                    DockerRepositoryTagAndDigestInfo {
                        tag_info: DockerRepositoryTagInfo {
                            tag: "beta".to_string(),
                            last_updated: Utc.timestamp_opt(0, 0).unwrap().into(),
                        },
                        digest: "sha256:d89e1bee20d9cb344674e213b581f14fbd8e70274ecf9d10c514bab78a307845".to_string(),
                    }
				],
			},
			&[
				Token::Struct {
                    name: "ListDockerRepositoryTagsResponse",
					len: 1,
				},
				Token::Str("tags"),
				Token::Seq { len: Some(2) },
				Token::Map {
					len: None,
				},
				Token::Str("tag"),
				Token::Str("stable"),
				Token::Str("lastUpdated"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("digest"),
				Token::Str("sha256:fea8895f450959fa676bcc1df0611ea93823a735a01205fd8622846041d0c7cf"),
				Token::MapEnd,
				Token::Map {
					len: None,
				},
				Token::Str("tag"),
				Token::Str("beta"),
				Token::Str("lastUpdated"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("digest"),
				Token::Str("sha256:d89e1bee20d9cb344674e213b581f14fbd8e70274ecf9d10c514bab78a307845"),
				Token::MapEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types_without_tags() {
		assert_tokens(
			&ApiResponse::success(ListDockerRepositoryTagsResponse {
				tags: vec![],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
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
			&ApiResponse::success(ListDockerRepositoryTagsResponse {
				tags: vec![
					DockerRepositoryTagAndDigestInfo {
                        tag_info: DockerRepositoryTagInfo {
                            tag: "stable".to_string(),
                            last_updated: Utc.timestamp_opt(0, 0).unwrap().into(),
                        },
                        digest: "sha256:fea8895f450959fa676bcc1df0611ea93823a735a01205fd8622846041d0c7cf".to_string(),
                    },
                    DockerRepositoryTagAndDigestInfo {
                        tag_info: DockerRepositoryTagInfo {
                            tag: "beta".to_string(),
                            last_updated: Utc.timestamp_opt(0, 0).unwrap().into(),
                        },
                        digest: "sha256:d89e1bee20d9cb344674e213b581f14fbd8e70274ecf9d10c514bab78a307845".to_string(),
                    }
				],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("tags"),
				Token::Seq { len: Some(2) },
				Token::Map {
					len: None,
				},
				Token::Str("tag"),
				Token::Str("stable"),
				Token::Str("lastUpdated"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("digest"),
				Token::Str("sha256:fea8895f450959fa676bcc1df0611ea93823a735a01205fd8622846041d0c7cf"),
				Token::MapEnd,
				Token::Map {
					len: None,
				},
				Token::Str("tag"),
				Token::Str("beta"),
				Token::Str("lastUpdated"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("digest"),
				Token::Str("sha256:d89e1bee20d9cb344674e213b581f14fbd8e70274ecf9d10c514bab78a307845"),
				Token::MapEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
