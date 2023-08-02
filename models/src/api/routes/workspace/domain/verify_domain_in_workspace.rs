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
#[typed_path("/workspace/:workspace_id/domain/:domain_id/verify")]
pub struct VerifyDomainPath {
	pub workspace_id: Uuid,
	pub domain_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VerifyDomainRequest;

impl ApiRequest for VerifyDomainRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = VerifyDomainPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = VerifyDomainResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VerifyDomainResponse {
	pub verified: bool,
}

#[cfg(test)]
mod test {

	use serde_test::{assert_tokens, Token};

	use super::{VerifyDomainRequest, VerifyDomainResponse};
	use crate::ApiResponse;

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&VerifyDomainRequest,
			&[Token::UnitStruct {
				name: "VerifyDomainRequest",
			}],
		)
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&VerifyDomainResponse { verified: true },
			&[
				Token::Struct {
					name: "VerifyDomainResponse",
					len: 1,
				},
				Token::Str("verified"),
				Token::Bool(true),
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(VerifyDomainResponse { verified: true }),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("verified"),
				Token::Bool(true),
				Token::MapEnd,
			],
		)
	}
}
