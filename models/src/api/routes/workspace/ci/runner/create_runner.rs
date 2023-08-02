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
#[typed_path("/workspace/:workspace_id/ci/runner")]
pub struct CreateRunnerPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateRunnerRequest {
	pub name: String,
	pub region_id: Uuid,
	pub build_machine_type_id: Uuid,
}

impl ApiRequest for CreateRunnerRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = CreateRunnerPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = CreateRunnerResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateRunnerResponse {
	pub id: Uuid,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{CreateRunnerRequest, CreateRunnerResponse};
	use crate::{utils::Uuid, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&CreateRunnerRequest {
				name: "runner name".into(),
				region_id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30869")
					.unwrap(),
				build_machine_type_id: Uuid::parse_str(
					"2aef18631ded45eb9170dc2166b30869",
				)
				.unwrap(),
			},
			&[
				Token::Struct {
					name: "CreateRunnerRequest",
					len: 3,
				},
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
	fn assert_response_types() {
		assert_tokens(
			&CreateRunnerResponse {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
			},
			&[
				Token::Struct {
					name: "CreateRunnerResponse",
					len: 1,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(CreateRunnerResponse {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::MapEnd,
			],
		)
	}
}
