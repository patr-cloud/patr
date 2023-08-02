use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::DockerRepositoryTagInfo;
use crate::{utils::Uuid, ApiRequest};

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
#[typed_path(
	"/workspace/:workspace_id/docker-registry/:repository_id/tag/:tag"
)]
pub struct GetDockerRepositoryTagDetailsPath {
	pub workspace_id: Uuid,
	pub repository_id: Uuid,
	pub tag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetDockerRepositoryTagDetailsRequest;

impl ApiRequest for GetDockerRepositoryTagDetailsRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetDockerRepositoryTagDetailsPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = GetDockerRepositoryTagDetailsResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetDockerRepositoryTagDetailsResponse {
	#[serde(flatten)]
	pub tag_info: DockerRepositoryTagInfo,
	pub digest: String,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{
		GetDockerRepositoryTagDetailsRequest,
		GetDockerRepositoryTagDetailsResponse,
	};
	use crate::{
		models::workspace::docker_registry::DockerRepositoryTagInfo,
		utils::DateTime,
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GetDockerRepositoryTagDetailsRequest,
			&[Token::UnitStruct {
				name: "GetDockerRepositoryTagDetailsRequest",
			}],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&GetDockerRepositoryTagDetailsResponse {
				tag_info: DockerRepositoryTagInfo {
					tag: "stable".to_string(),
					last_updated: DateTime::default(),
				},
				digest: "sha256:fea8895f450959fa676bcc1df0611ea93823a735a01205fd8622846041d0c7cf".to_string(),
			},
			&[
				Token::Map { len: None },
				Token::Str("tag"),
				Token::Str("stable"),
				Token::Str("lastUpdated"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("digest"),
				Token::Str("sha256:fea8895f450959fa676bcc1df0611ea93823a735a01205fd8622846041d0c7cf"),
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GetDockerRepositoryTagDetailsResponse {
				tag_info: DockerRepositoryTagInfo {
					tag: "stable".to_string(),
					last_updated: DateTime::default(),
				},
				digest: "sha256:fea8895f450959fa676bcc1df0611ea93823a735a01205fd8622846041d0c7cf".to_string(),
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("tag"),
				Token::Str("stable"),
				Token::Str("lastUpdated"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("digest"),
				Token::Str("sha256:fea8895f450959fa676bcc1df0611ea93823a735a01205fd8622846041d0c7cf"),
				Token::MapEnd,
			],
		);
	}
}
