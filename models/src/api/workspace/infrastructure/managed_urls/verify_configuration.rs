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
#[typed_path("/workspace/:workspace_id/infrastructure/managed-url/:managed_url_id/verify-configuration")]
pub struct VerifyManagedUrlConfigurationPath {
	pub workspace_id: Uuid,
	pub managed_url_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VerifyManagedUrlConfigurationRequest {}

impl ApiRequest for VerifyManagedUrlConfigurationRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = VerifyManagedUrlConfigurationPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = VerifyManagedUrlConfigurationResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VerifyManagedUrlConfigurationResponse {
	pub configured: bool,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{
		VerifyManagedUrlConfigurationRequest,
		VerifyManagedUrlConfigurationResponse,
	};
	use crate::ApiResponse;

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&VerifyManagedUrlConfigurationRequest {},
			&[
				Token::Struct {
					name: "VerifyManagedUrlConfigurationRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&VerifyManagedUrlConfigurationResponse { configured: true },
			&[
				Token::Struct {
					name: "VerifyManagedUrlConfigurationResponse",
					len: 1,
				},
				Token::Str("configured"),
				Token::Bool(true),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(VerifyManagedUrlConfigurationResponse {
				configured: true,
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("configured"),
				Token::Bool(true),
				Token::MapEnd,
			],
		);
	}
}
