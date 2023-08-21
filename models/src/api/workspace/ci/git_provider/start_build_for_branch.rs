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
#[typed_path("/workspace/:workspace_id/ci/git-provider/github/repo/:repo_id/branch/:branch_name/start")]
pub struct StartBuildPath {
	pub workspace_id: Uuid,
	pub repo_id: String,
	pub branch_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StartBuildRequest {}

impl ApiRequest for StartBuildRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = StartBuildPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = StartBuildResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StartBuildResponse {
	pub build_num: u64,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{StartBuildRequest, StartBuildResponse};
	use crate::ApiResponse;

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&StartBuildRequest {},
			&[
				Token::Struct {
					name: "StartBuildRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&StartBuildResponse { build_num: 42 },
			&[
				Token::Struct {
					name: "StartBuildResponse",
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
			&ApiResponse::success(StartBuildResponse { build_num: 42 }),
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
