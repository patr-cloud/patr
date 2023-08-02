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
#[typed_path("/workspace/:workspace_id/secret/:secret_id")]
pub struct DeleteSecretPath {
	pub workspace_id: Uuid,
	pub secret_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeleteSecretRequest;

impl ApiRequest for DeleteSecretRequest {
	const METHOD: Method = Method::DELETE;
	const IS_PROTECTED: bool = true;

	type RequestPath = DeleteSecretPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = ();
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::DeleteSecretRequest;
	use crate::{ApiRequest, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&DeleteSecretRequest,
			&[Token::UnitStruct {
				name: "DeleteSecretRequest",
			}],
		);
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<<DeleteSecretRequest as ApiRequest>::Response>(());
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
		);
	}
}
