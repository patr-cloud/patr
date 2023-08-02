use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::ApiRequest;

#[derive(
	Eq,
	Ord,
	Copy,
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
#[typed_path("/user/update-recovery-email")]
pub struct UpdateRecoveryEmailPath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRecoveryEmailRequest {
	pub recovery_email: String,
}

impl ApiRequest for UpdateRecoveryEmailRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = UpdateRecoveryEmailPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = ();
}

#[cfg(test)]
mod tests {
	use serde_test::{assert_tokens, Token};

	use super::UpdateRecoveryEmailRequest;
	use crate::{ApiRequest, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&UpdateRecoveryEmailRequest {
				recovery_email: "johnpatr@gmail.com".to_string(),
			},
			&[
				Token::Struct {
					name: "UpdateRecoveryEmailRequest",
					len: 1,
				},
				Token::Str("recoveryEmail"),
				Token::Str("johnpatr@gmail.com"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<
			<UpdateRecoveryEmailRequest as ApiRequest>::Response,
		>(());
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
