use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::BuildLogs;
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
#[typed_path("/workspace/:workspace_id/ci/git-provider/github/repo/:repo_id/build/:build_num/log/:step")]
pub struct GetBuildLogPath {
	pub workspace_id: Uuid,
	pub repo_id: String,
	pub build_num: u64,
	pub step: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetBuildLogRequest {}

impl ApiRequest for GetBuildLogRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetBuildLogPath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = GetBuildLogResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetBuildLogResponse {
	pub logs: Vec<BuildLogs>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{BuildLogs, GetBuildLogRequest, GetBuildLogResponse};
	use crate::ApiResponse;

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GetBuildLogRequest {},
			&[
				Token::Struct {
					name: "GetBuildLogRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&GetBuildLogResponse {
				logs: vec![
					BuildLogs {
						log: "+ git init\n".to_string(),
						time: 1,
					},
					BuildLogs {
						log: "+ git submodule update --init\n".to_string(),
						time: 2,
					},
				],
			},
			&[
				Token::Struct {
					name: "GetBuildLogResponse",
					len: 1,
				},
				Token::Str("logs"),
				Token::Seq { len: Some(2) },
				Token::Struct {
					name: "BuildLogs",
					len: 2,
				},
				Token::Str("log"),
				Token::Str("+ git init\n"),
				Token::Str("time"),
				Token::U64(1),
				Token::StructEnd,
				Token::Struct {
					name: "BuildLogs",
					len: 2,
				},
				Token::Str("log"),
				Token::Str("+ git submodule update --init\n"),
				Token::Str("time"),
				Token::U64(2),
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GetBuildLogResponse {
				logs: vec![
					BuildLogs {
						log: "+ git init\n".to_string(),
						time: 1,
					},
					BuildLogs {
						log: "+ git submodule update --init\n".to_string(),
						time: 2,
					},
				],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("logs"),
				Token::Seq { len: Some(2) },
				Token::Struct {
					name: "BuildLogs",
					len: 2,
				},
				Token::Str("log"),
				Token::Str("+ git init\n"),
				Token::Str("time"),
				Token::U64(1),
				Token::StructEnd,
				Token::Struct {
					name: "BuildLogs",
					len: 2,
				},
				Token::Str("log"),
				Token::Str("+ git submodule update --init\n"),
				Token::Str("time"),
				Token::U64(2),
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
