use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::DockerRepository;
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
#[typed_path("/workspace/:workspace_id/docker-registry")]
pub struct ListDockerRepositoriesPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListDockerRepositoriesRequest;

impl ApiRequest for ListDockerRepositoriesRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListDockerRepositoriesPath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = ListDockerRepositoriesResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListDockerRepositoriesResponse {
	pub repositories: Vec<DockerRepository>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{
		ListDockerRepositoriesRequest,
		ListDockerRepositoriesResponse,
	};
	use crate::{
		models::workspace::docker_registry::DockerRepository,
		utils::{DateTime, Uuid},
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ListDockerRepositoriesRequest,
			&[Token::UnitStruct {
				name: "ListDockerRepositoriesRequest",
			}],
		);
	}

	#[test]
	fn assert_response_types_without_repositories() {
		assert_tokens(
			&ListDockerRepositoriesResponse {
				repositories: vec![],
			},
			&[
				Token::Struct {
					name: "ListDockerRepositoriesResponse",
					len: 1,
				},
				Token::Str("repositories"),
				Token::Seq { len: Some(0) },
				Token::SeqEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types_with_repositories() {
		assert_tokens(
			&ListDockerRepositoriesResponse {
				repositories: vec![
					DockerRepository {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						name: "test".to_string(),
						size: 1234567890,
						last_updated: DateTime::default(),
					},
					DockerRepository {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30868")
							.unwrap(),
						name: "test2".to_string(),
						size: 1234567890,
						last_updated: DateTime::default(),
					},
				],
			},
			&[
				Token::Struct {
					name: "ListDockerRepositoriesResponse",
					len: 1,
				},
				Token::Str("repositories"),
				Token::Seq { len: Some(2) },
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
				Token::Struct {
					name: "DockerRepository",
					len: 4,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Str("name"),
				Token::Str("test2"),
				Token::Str("size"),
				Token::U64(1234567890),
				Token::Str("lastUpdated"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types_without_repositories() {
		assert_tokens(
			&ApiResponse::success(ListDockerRepositoriesResponse {
				repositories: vec![],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("repositories"),
				Token::Seq { len: Some(0) },
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types_with_repositories() {
		assert_tokens(
			&ApiResponse::success(ListDockerRepositoriesResponse {
				repositories: vec![
					DockerRepository {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						name: "test".to_string(),
						size: 1234567890,
						last_updated: DateTime::default(),
					},
					DockerRepository {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30868")
							.unwrap(),
						name: "test2".to_string(),
						size: 1234567890,
						last_updated: DateTime::default(),
					},
				],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("repositories"),
				Token::Seq { len: Some(2) },
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
				Token::Struct {
					name: "DockerRepository",
					len: 4,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Str("name"),
				Token::Str("test2"),
				Token::Str("size"),
				Token::U64(1234567890),
				Token::Str("lastUpdated"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
