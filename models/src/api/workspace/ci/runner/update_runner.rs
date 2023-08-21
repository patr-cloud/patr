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
#[typed_path("/workspace/:workspace_id/ci/runner/:runner_id")]
pub struct UpdateRunnerPath {
	pub workspace_id: Uuid,
	pub runner_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRunnerRequest {
	pub name: String,
}

impl ApiRequest for UpdateRunnerRequest {
	const METHOD: Method = Method::PATCH;
	const IS_PROTECTED: bool = true;

	type RequestPath = UpdateRunnerPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = ();
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::UpdateRunnerRequest;
	use crate::{ApiRequest, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&UpdateRunnerRequest {
				name: "patr".to_string(),
			},
			&[
				Token::Struct {
					name: "UpdateRunnerRequest",
					len: 1,
				},
				Token::Str("name"),
				Token::Str("patr"),
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<<UpdateRunnerRequest as ApiRequest>::Response>(());
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(()),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::MapEnd,
			],
		)
	}
}
