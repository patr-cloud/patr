use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::Runner;
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
#[typed_path("/workspace/:workspace_id/ci/runner/:runner_id")]
pub struct GetRunnerInfoPath {
	pub workspace_id: Uuid,
	pub runner_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetRunnerInfoRequest {}

impl ApiRequest for GetRunnerInfoRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetRunnerInfoPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = GetRunnerInfoResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetRunnerInfoResponse(pub Runner);

#[cfg(test)]
mod test {

	use serde_test::{assert_tokens, Token};

	use super::{GetRunnerInfoRequest, GetRunnerInfoResponse};
	use crate::{
		models::workspace::ci::runner::Runner,
		utils::Uuid,
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GetRunnerInfoRequest {},
			&[
				Token::Struct {
					name: "GetRunnerInfoRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&GetRunnerInfoResponse(Runner {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
				name: "runner name".into(),
				region_id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30869")
					.unwrap(),
				build_machine_type_id: Uuid::parse_str(
					"2aef18631ded45eb9170dc2166b30869",
				)
				.unwrap(),
			}),
			&[
				Token::NewtypeStruct {
					name: "GetRunnerInfoResponse",
				},
				Token::Struct {
					name: "Runner",
					len: 4,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("runner name"),
				Token::Str("regionId"),
				Token::Str("2aef18631ded45eb9170dc2166b30869"),
				Token::Str("buildMachineTypeId"),
				Token::Str("2aef18631ded45eb9170dc2166b30869"),
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GetRunnerInfoResponse(Runner {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
				name: "runner name".into(),
				region_id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30869")
					.unwrap(),
				build_machine_type_id: Uuid::parse_str(
					"2aef18631ded45eb9170dc2166b30869",
				)
				.unwrap(),
			})),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("runner name"),
				Token::Str("regionId"),
				Token::Str("2aef18631ded45eb9170dc2166b30869"),
				Token::Str("buildMachineTypeId"),
				Token::Str("2aef18631ded45eb9170dc2166b30869"),
				Token::MapEnd,
			],
		)
	}
}
