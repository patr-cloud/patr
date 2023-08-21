use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
#[typed_path("/survey")]
pub struct SubmitSurveyPath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SubmitSurveyRequest {
	pub survey_response: Value,
	pub version: String,
}

impl ApiRequest for SubmitSurveyRequest {
	const IS_PROTECTED: bool = true;
	const METHOD: Method = Method::POST;
	
	type RequestPath = SubmitSurveyPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = ();
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SubmitSurveyResponse {}

#[cfg(test)]
mod test {
	use serde_json::Value;
	use serde_test::{assert_tokens, Token};

	use super::{SubmitSurveyRequest, SubmitSurveyResponse};
	use crate::ApiResponse;

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&SubmitSurveyRequest {
				survey_response: Value::from("{\"version\": \"v1\"}"),
				version: "v1".to_string(),
			},
			&[
				Token::Struct {
					name: "SubmitSurveyRequest",
					len: 2,
				},
				Token::Str("surveyResponse"),
				Token::Str("{\"version\": \"v1\"}"),
				Token::Str("version"),
				Token::Str("v1"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&SubmitSurveyResponse {},
			&[
				Token::Struct {
					name: "SubmitSurveyResponse",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(SubmitSurveyResponse {}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::MapEnd,
			],
		);
	}
}
