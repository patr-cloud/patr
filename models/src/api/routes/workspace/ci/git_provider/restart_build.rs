use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

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
#[typed_path("/workspace/:workspace_id/ci/git-provider/github/repo/:repo_id/build/:build_num/restart")]
pub struct RestartBuildPath {
	pub workspace_id: Uuid,
	pub repo_id: String,
	pub build_num: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RestartBuildRequest {}

impl ApiRequest for RestartBuildRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = RestartBuildPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = RestartBuildResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RestartBuildResponse {
	pub build_num: u64,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{RestartBuildRequest, RestartBuildResponse};
	use crate::ApiResponse;

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&RestartBuildRequest {},
			&[
				Token::Struct {
					name: "RestartBuildRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&RestartBuildResponse { build_num: 42 },
			&[
				Token::Struct {
					name: "RestartBuildResponse",
					len: 1,
				},
				Token::Str("buildNum"),
				Token::U64(42),
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(RestartBuildResponse { build_num: 42 }),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("buildNum"),
				Token::U64(42),
				Token::MapEnd,
			],
		);
	}
}
