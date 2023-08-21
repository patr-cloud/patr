use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::ApiRequest;

#[derive(
	Eq,
	Ord,
	Hash,
	Copy,
	Debug,
	Clone,
	Default,
	TypedPath,
	PartialEq,
	Serialize,
	PartialOrd,
	Deserialize,
)]
#[typed_path("/workspace/is-name-available")]
pub struct IsWorkspaceNameAvailablePath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IsWorkspaceNameAvailableRequest {
	pub name: String,
}

impl ApiRequest for IsWorkspaceNameAvailableRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = false;

	type RequestPath = IsWorkspaceNameAvailablePath;
	type RequestQuery = Self;
	type RequestBody = ();
	type Response = IsWorkspaceNameAvailableResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IsWorkspaceNameAvailableResponse {
	pub available: bool,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{
		IsWorkspaceNameAvailableRequest,
		IsWorkspaceNameAvailableResponse,
	};
	use crate::ApiResponse;

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&IsWorkspaceNameAvailableRequest {
				name: "John Patr's Company".to_string(),
			},
			&[
				Token::Struct {
					name: "IsWorkspaceNameAvailableRequest",
					len: 1,
				},
				Token::Str("name"),
				Token::Str("John Patr's Company"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types_available() {
		assert_tokens(
			&IsWorkspaceNameAvailableResponse { available: true },
			&[
				Token::Struct {
					name: "IsWorkspaceNameAvailableResponse",
					len: 1,
				},
				Token::Str("available"),
				Token::Bool(true),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types_not_available() {
		assert_tokens(
			&IsWorkspaceNameAvailableResponse { available: false },
			&[
				Token::Struct {
					name: "IsWorkspaceNameAvailableResponse",
					len: 1,
				},
				Token::Str("available"),
				Token::Bool(false),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types_available() {
		assert_tokens(
			&ApiResponse::success(IsWorkspaceNameAvailableResponse {
				available: true,
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("available"),
				Token::Bool(true),
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types_not_available() {
		assert_tokens(
			&ApiResponse::success(IsWorkspaceNameAvailableResponse {
				available: false,
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("available"),
				Token::Bool(false),
				Token::MapEnd,
			],
		);
	}
}
