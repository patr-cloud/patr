use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::Runner;
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
#[typed_path("/workspace/:workspace_id/ci/runner")]
pub struct ListCiRunnerPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListCiRunnerRequest {}

impl ApiRequest for ListCiRunnerRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListCiRunnerPath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = ListCiRunnerResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListCiRunnerResponse {
	pub runners: Vec<Runner>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{ListCiRunnerRequest, ListCiRunnerResponse};
	use crate::{
		models::workspace::ci::runner::Runner,
		utils::Uuid,
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ListCiRunnerRequest {},
			&[
				Token::Struct {
					name: "ListCiRunnerRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&ListCiRunnerResponse {
				runners: vec![Runner {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					name: "runner name".into(),
					region_id: Uuid::parse_str(
						"2aef18631ded45eb9170dc2166b30869",
					)
					.unwrap(),
					build_machine_type_id: Uuid::parse_str(
						"2aef18631ded45eb9170dc2166b30869",
					)
					.unwrap(),
				}],
			},
			&[
				Token::Struct {
					name: "ListCiRunnerResponse",
					len: 1,
				},
				Token::Str("runners"),
				Token::Seq { len: Some(1) },
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
				Token::SeqEnd,
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(ListCiRunnerResponse {
				runners: vec![Runner {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					name: "runner name".into(),
					region_id: Uuid::parse_str(
						"2aef18631ded45eb9170dc2166b30869",
					)
					.unwrap(),
					build_machine_type_id: Uuid::parse_str(
						"2aef18631ded45eb9170dc2166b30869",
					)
					.unwrap(),
				}],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("runners"),
				Token::Seq { len: Some(1) },
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
				Token::SeqEnd,
				Token::MapEnd,
			],
		)
	}
}
