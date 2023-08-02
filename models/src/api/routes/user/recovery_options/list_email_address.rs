use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::{utils::Paginated, ApiRequest};

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
#[typed_path("/user/list-email-address")]
pub struct ListPersonalEmailsPath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListPersonalEmailsRequest;

impl ApiRequest for ListPersonalEmailsRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListPersonalEmailsPath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = ListPersonalEmailsResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListPersonalEmailsResponse {
	pub recovery_email: Option<String>,
	pub secondary_emails: Vec<String>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{ListPersonalEmailsRequest, ListPersonalEmailsResponse};
	use crate::ApiResponse;

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ListPersonalEmailsRequest,
			&[Token::UnitStruct {
				name: "ListPersonalEmailsRequest",
			}],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&ListPersonalEmailsResponse {
				recovery_email: Some("johnpatr@gmail.com".to_string()),
				secondary_emails: vec!["johnpatr1@gmail.com".to_string()],
			},
			&[
				Token::Struct {
					name: "ListPersonalEmailsResponse",
					len: 2,
				},
				Token::Str("recoveryEmail"),
				Token::Some,
				Token::Str("johnpatr@gmail.com"),
				Token::Str("secondaryEmails"),
				Token::Seq { len: Some(1) },
				Token::Str("johnpatr1@gmail.com"),
				Token::SeqEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(ListPersonalEmailsResponse {
				recovery_email: Some("johnpatr@gmail.com".to_string()),
				secondary_emails: vec!["johnpatr1@gmail.com".to_string()],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("recoveryEmail"),
				Token::Some,
				Token::Str("johnpatr@gmail.com"),
				Token::Str("secondaryEmails"),
				Token::Seq { len: Some(1) },
				Token::Str("johnpatr1@gmail.com"),
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
