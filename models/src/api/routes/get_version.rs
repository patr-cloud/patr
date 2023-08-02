#[derive(
	Eq,
	Hash,
	Debug,
	Clone,
	PartialEq,
	PartialOrd,
	serde::Serialize,
	serde::Deserialize,
	axum_extra::routing::TypedPath,
)]
#[typed_path("/version")]
pub struct GetVersionPath;

#[derive(Eq, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetVersionRequest;

impl crate::api::ApiEndpoint for GetVersionRequest {
	const METHOD: reqwest::Method = reqwest::Method::GET;
	type RequestPath = GetVersionPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type ResponseBody = GetVersionResponse;
}
#[derive(Eq, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetVersionResponse;

// #[cfg(test)]
// mod test {
// 	use serde_test::{assert_tokens, Token};

// 	use super::{GetVersionRequest, GetVersionResponse};
// 	use crate::ApiResponse;

// 	#[test]
// 	fn assert_request_types() {
// 		assert_tokens(
// 			&GetVersionRequest,
// 			&[Token::UnitStruct {
// 				name: "GetVersionRequest",
// 			}],
// 		);
// 	}

// 	#[test]
// 	fn assert_response_types() {
// 		assert_tokens(
// 			&GetVersionResponse {
// 				version: "0.1.0".to_string(),
// 			},
// 			&[
// 				Token::Struct {
// 					name: "GetVersionResponse",
// 					len: 1,
// 				},
// 				Token::Str("version"),
// 				Token::Str("0.1.0"),
// 				Token::StructEnd,
// 			],
// 		);
// 	}

// 	#[test]
// 	fn assert_success_response_types() {
// 		assert_tokens(
// 			&ApiResponse::success(GetVersionResponse {
// 				version: "0.1.0".to_string(),
// 			}),
// 			&[
// 				Token::Map { len: None },
// 				Token::Str("success"),
// 				Token::Bool(true),
// 				Token::Str("version"),
// 				Token::Str("0.1.0"),
// 				Token::MapEnd,
// 			],
// 		);
// 	}
// }
