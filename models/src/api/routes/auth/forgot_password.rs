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
#[typed_path("/auth/forgot-password")]
pub struct ForgotPasswordPath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ForgotPasswordRequest {
	pub user_id: String,
	pub preferred_recovery_option: PreferredRecoveryOption,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PreferredRecoveryOption {
	RecoveryPhoneNumber,
	RecoveryEmail,
}

impl ApiRequest for ForgotPasswordRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = false;

	type RequestPath = ForgotPasswordPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = ();
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{ForgotPasswordRequest, PreferredRecoveryOption};
	use crate::{ApiRequest, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ForgotPasswordRequest {
				user_id: "john-patr".to_string(),
				preferred_recovery_option:
					PreferredRecoveryOption::RecoveryPhoneNumber,
			},
			&[
				Token::Struct {
					name: "ForgotPasswordRequest",
					len: 2,
				},
				Token::Str("userId"),
				Token::Str("john-patr"),
				Token::Str("preferredRecoveryOption"),
				Token::UnitVariant {
					name: "PreferredRecoveryOption",
					variant: "recoveryPhoneNumber",
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<<ForgotPasswordRequest as ApiRequest>::Response>(
			(),
		);
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
