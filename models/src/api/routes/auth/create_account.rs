use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::{
	utils::{Business, Personal, ResourceType},
	ApiRequest,
};

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
#[typed_path("/auth/sign-up")]
pub struct CreateAccountPath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateAccountRequest {
	pub username: String,
	pub password: String,
	pub first_name: String,
	pub last_name: String,
	#[serde(flatten)]
	pub recovery_method: RecoveryMethod,
	#[serde(flatten)]
	pub account_type: SignUpAccountType,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub coupon_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum RecoveryMethod {
	#[serde(rename_all = "camelCase")]
	PhoneNumber {
		recovery_phone_country_code: String,
		recovery_phone_number: String,
	},
	#[serde(rename_all = "camelCase")]
	Email { recovery_email: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum SignUpAccountType {
	#[serde(rename_all = "camelCase")]
	Personal { account_type: Personal },
	#[serde(rename_all = "camelCase")]
	Business {
		account_type: Business,
		workspace_name: String,
		business_email_local: String,
		domain: String,
	},
}

impl SignUpAccountType {
	pub fn is_personal(&self) -> bool {
		matches!(self, Self::Personal { .. })
	}

	pub fn is_business(&self) -> bool {
		matches!(self, Self::Business { .. })
	}
}

impl SignUpAccountType {
	pub fn account_type(&self) -> ResourceType {
		match self {
			Self::Personal { .. } => ResourceType::Personal,
			Self::Business { .. } => ResourceType::Business,
		}
	}
}

impl ApiRequest for CreateAccountRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = false;

	type RequestPath = CreateAccountPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = ();
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{CreateAccountRequest, RecoveryMethod, SignUpAccountType};
	use crate::{
		utils::{Business, Personal},
		ApiRequest,
		ApiResponse,
	};

	#[test]
	fn assert_request_types_for_personal_account_with_email() {
		assert_tokens(
			&CreateAccountRequest {
				username: "john-patr".to_string(),
				password: "hunter42".to_string(),
				first_name: "John".to_string(),
				last_name: "Patr".to_string(),
				recovery_method: RecoveryMethod::Email {
					recovery_email: "johnpatr@gmail.com".to_string(),
				},
				account_type: SignUpAccountType::Personal {
					account_type: Personal,
				},
				coupon_code: None,
			},
			&[
				Token::Map { len: None },
				Token::Str("username"),
				Token::Str("john-patr"),
				Token::Str("password"),
				Token::Str("hunter42"),
				Token::Str("firstName"),
				Token::Str("John"),
				Token::Str("lastName"),
				Token::Str("Patr"),
				Token::Str("recoveryEmail"),
				Token::Str("johnpatr@gmail.com"),
				Token::Str("accountType"),
				Token::Str("personal"),
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_request_types_for_personal_account_with_phone_number() {
		assert_tokens(
			&CreateAccountRequest {
				username: "john-patr".to_string(),
				password: "hunter42".to_string(),
				first_name: "John".to_string(),
				last_name: "Patr".to_string(),
				recovery_method: RecoveryMethod::PhoneNumber {
					recovery_phone_country_code: "IN".to_string(),
					recovery_phone_number: "1234567890".to_string(),
				},
				account_type: SignUpAccountType::Personal {
					account_type: Personal,
				},
				coupon_code: None,
			},
			&[
				Token::Map { len: None },
				Token::Str("username"),
				Token::Str("john-patr"),
				Token::Str("password"),
				Token::Str("hunter42"),
				Token::Str("firstName"),
				Token::Str("John"),
				Token::Str("lastName"),
				Token::Str("Patr"),
				Token::Str("recoveryPhoneCountryCode"),
				Token::Str("IN"),
				Token::Str("recoveryPhoneNumber"),
				Token::Str("1234567890"),
				Token::Str("accountType"),
				Token::Str("personal"),
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_request_types_for_business_account_with_email() {
		assert_tokens(
			&CreateAccountRequest {
				username: "john-patr".to_string(),
				password: "hunter42".to_string(),
				first_name: "John".to_string(),
				last_name: "Patr".to_string(),
				recovery_method: RecoveryMethod::Email {
					recovery_email: "johnpatr@gmail.com".to_string(),
				},
				account_type: SignUpAccountType::Business {
					account_type: Business,
					workspace_name: "Patr Co".to_string(),
					business_email_local: "johnpatr".to_string(),
					domain: "johnpatr.com".to_string(),
				},
				coupon_code: None,
			},
			&[
				Token::Map { len: None },
				Token::Str("username"),
				Token::Str("john-patr"),
				Token::Str("password"),
				Token::Str("hunter42"),
				Token::Str("firstName"),
				Token::Str("John"),
				Token::Str("lastName"),
				Token::Str("Patr"),
				Token::Str("recoveryEmail"),
				Token::Str("johnpatr@gmail.com"),
				Token::Str("accountType"),
				Token::Str("business"),
				Token::Str("workspaceName"),
				Token::Str("Patr Co"),
				Token::Str("businessEmailLocal"),
				Token::Str("johnpatr"),
				Token::Str("domain"),
				Token::Str("johnpatr.com"),
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_request_types_for_business_account_with_phone_number() {
		assert_tokens(
			&CreateAccountRequest {
				username: "john-patr".to_string(),
				password: "hunter42".to_string(),
				first_name: "John".to_string(),
				last_name: "Patr".to_string(),
				recovery_method: RecoveryMethod::PhoneNumber {
					recovery_phone_country_code: "IN".to_string(),
					recovery_phone_number: "1234567890".to_string(),
				},
				account_type: SignUpAccountType::Business {
					account_type: Business,
					workspace_name: "Patr Co".to_string(),
					business_email_local: "johnpatr".to_string(),
					domain: "johnpatr.com".to_string(),
				},
				coupon_code: None,
			},
			&[
				Token::Map { len: None },
				Token::Str("username"),
				Token::Str("john-patr"),
				Token::Str("password"),
				Token::Str("hunter42"),
				Token::Str("firstName"),
				Token::Str("John"),
				Token::Str("lastName"),
				Token::Str("Patr"),
				Token::Str("recoveryPhoneCountryCode"),
				Token::Str("IN"),
				Token::Str("recoveryPhoneNumber"),
				Token::Str("1234567890"),
				Token::Str("accountType"),
				Token::Str("business"),
				Token::Str("workspaceName"),
				Token::Str("Patr Co"),
				Token::Str("businessEmailLocal"),
				Token::Str("johnpatr"),
				Token::Str("domain"),
				Token::Str("johnpatr.com"),
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<<CreateAccountRequest as ApiRequest>::Response>(
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
