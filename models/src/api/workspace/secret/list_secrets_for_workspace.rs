use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::Secret;
use crate::{
	utils::{Paginated, Uuid},
	ApiRequest,
};

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
#[typed_path("/workspace/:workspace_id/secret")]
pub struct ListSecretsPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListSecretsRequest;

impl ApiRequest for ListSecretsRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListSecretsPath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = ListSecretsResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListSecretsResponse {
	pub secrets: Vec<Secret>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{ListSecretsRequest, ListSecretsResponse};
	use crate::{models::workspace::secret::Secret, utils::Uuid, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ListSecretsRequest,
			&[Token::UnitStruct {
				name: "ListSecretsRequest",
			}],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&ListSecretsResponse {
				secrets: vec![
					Secret {
						id: Uuid::parse_str("2bef29641dfd45eb9270dc2166c41867")
							.unwrap(),
						name: "test".to_string(),
						deployment_id: None,
					},
					Secret {
						id: Uuid::parse_str("2bef29641dfd45eb9270dc2166c41868")
							.unwrap(),
						name: "test".to_string(),
						deployment_id: Some(
							Uuid::parse_str("2bef29641dfd45eb9270dc2166c41869")
								.unwrap(),
						),
					},
				],
			},
			&[
				Token::Struct {
					name: "ListSecretsResponse",
					len: 1,
				},
				Token::Str("secrets"),
				Token::Seq { len: Some(2) },
				Token::Struct {
					name: "Secret",
					len: 2,
				},
				Token::Str("id"),
				Token::Str("2bef29641dfd45eb9270dc2166c41867"),
				Token::Str("name"),
				Token::Str("test"),
				Token::StructEnd,
				Token::Struct {
					name: "Secret",
					len: 3,
				},
				Token::Str("id"),
				Token::Str("2bef29641dfd45eb9270dc2166c41868"),
				Token::Str("name"),
				Token::Str("test"),
				Token::Str("deploymentId"),
				Token::Some,
				Token::Str("2bef29641dfd45eb9270dc2166c41869"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(ListSecretsResponse {
				secrets: vec![
					Secret {
						id: Uuid::parse_str("2bef29641dfd45eb9270dc2166c41867")
							.unwrap(),
						name: "test".to_string(),
						deployment_id: None,
					},
					Secret {
						id: Uuid::parse_str("2bef29641dfd45eb9270dc2166c41868")
							.unwrap(),
						name: "test".to_string(),
						deployment_id: Some(
							Uuid::parse_str("2bef29641dfd45eb9270dc2166c41869")
								.unwrap(),
						),
					},
				],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("secrets"),
				Token::Seq { len: Some(2) },
				Token::Struct {
					name: "Secret",
					len: 2,
				},
				Token::Str("id"),
				Token::Str("2bef29641dfd45eb9270dc2166c41867"),
				Token::Str("name"),
				Token::Str("test"),
				Token::StructEnd,
				Token::Struct {
					name: "Secret",
					len: 3,
				},
				Token::Str("id"),
				Token::Str("2bef29641dfd45eb9270dc2166c41868"),
				Token::Str("name"),
				Token::Str("test"),
				Token::Str("deploymentId"),
				Token::Some,
				Token::Str("2bef29641dfd45eb9270dc2166c41869"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
