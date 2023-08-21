use std::collections::BTreeMap;

use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::{
	models::workspace::infrastructure::deployment::ExposedPortType,
	utils::{StringifiedU16, Uuid},
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
#[typed_path(
	"/workspace/:workspace_id/docker-registry/:repository_id/exposed-ports"
)]
pub struct GetDockerRepositoryExposedPortPath {
	pub workspace_id: Uuid,
	pub repository_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetDockerRepositoryExposedPortRequest {
	pub tag: String,
}

impl ApiRequest for GetDockerRepositoryExposedPortRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetDockerRepositoryExposedPortPath;
	type RequestQuery = Self;
	type RequestBody = ();
	type Response = GetDockerRepositoryExposedPortResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetDockerRepositoryExposedPortResponse {
	pub ports: BTreeMap<StringifiedU16, ExposedPortType>,
}

#[cfg(test)]
mod test {
	use std::collections::BTreeMap;

	use serde_test::{assert_tokens, Token};

	use super::{
		GetDockerRepositoryExposedPortRequest,
		GetDockerRepositoryExposedPortResponse,
	};
	use crate::{
		models::workspace::infrastructure::deployment::ExposedPortType,
		utils::StringifiedU16,
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GetDockerRepositoryExposedPortRequest {
				tag: "latest".to_string(),
			},
			&[
				Token::Struct {
					name: "GetDockerRepositoryExposedPortRequest",
					len: 1,
				},
				Token::Str("tag"),
				Token::Str("latest"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&GetDockerRepositoryExposedPortResponse {
				ports: {
					let mut map = BTreeMap::new();

					map.insert(
						StringifiedU16::new(3000),
						ExposedPortType::Http,
					);
					map.insert(StringifiedU16::new(8080), ExposedPortType::Tcp);

					map
				},
			},
			&[
				Token::Struct {
					name: "GetDockerRepositoryExposedPortResponse",
					len: 1,
				},
				Token::Str("ports"),
				Token::Map { len: Some(2) },
				Token::Str("3000"),
				Token::Struct {
					name: "ExposedPortType",
					len: 1,
				},
				Token::Str("type"),
				Token::Str("http"),
				Token::StructEnd,
				Token::Str("8080"),
				Token::Struct {
					name: "ExposedPortType",
					len: 1,
				},
				Token::Str("type"),
				Token::Str("tcp"),
				Token::StructEnd,
				Token::MapEnd,
				Token::StructEnd,
			],
		);
	}
	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GetDockerRepositoryExposedPortResponse {
				ports: {
					let mut map = BTreeMap::new();

					map.insert(
						StringifiedU16::new(3000),
						ExposedPortType::Http,
					);
					map.insert(StringifiedU16::new(8080), ExposedPortType::Tcp);

					map
				},
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("ports"),
				Token::Map { len: Some(2) },
				Token::Str("3000"),
				Token::Struct {
					name: "ExposedPortType",
					len: 1,
				},
				Token::Str("type"),
				Token::Str("http"),
				Token::StructEnd,
				Token::Str("8080"),
				Token::Struct {
					name: "ExposedPortType",
					len: 1,
				},
				Token::Str("type"),
				Token::Str("tcp"),
				Token::StructEnd,
				Token::MapEnd,
				Token::MapEnd,
			],
		);
	}
}
