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
pub struct UpdateWorkspaceSecretPath {
	pub workspace_id: Uuid,
	pub secret_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWorkspaceSecretRequest {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub value: Option<String>,
}

impl ApiRequest for UpdateWorkspaceSecretRequest {
	const METHOD: Method = Method::PATCH;
	const IS_PROTECTED: bool = true;

	type RequestPath = UpdateWorkspaceSecretPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = ();
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::UpdateWorkspaceSecretRequest;
	use crate::{ApiRequest, ApiResponse};

	#[test]
	fn assert_empty_request_types() {
		assert_tokens(
			&UpdateWorkspaceSecretRequest {
				name: None,
				value: None,
			},
			&[
				Token::Struct {
					name: "UpdateWorkspaceSecretRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_valued_request_types() {
		assert_tokens(
			&UpdateWorkspaceSecretRequest {
				name: Some("test".to_string()),
				value: Some("asdf;lkj".to_string()),
			},
			&[
				Token::Struct {
					name: "UpdateWorkspaceSecretRequest",
					len: 2,
				},
				Token::Str("name"),
				Token::Some,
				Token::Str("test"),
				Token::Str("value"),
				Token::Some,
				Token::Str("asdf;lkj"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<
			<UpdateWorkspaceSecretRequest as ApiRequest>::Response,
		>(());
	}

	#[test]
	fn assert_success_response() {
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
