use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::{BuildLog, Interval};
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
#[typed_path(
	"/workspace/:workspace_id/infrastructure/deployment/:deployment_id/build-logs"
)]
pub struct GetDeploymentBuildLogsPath {
	pub workspace_id: Uuid,
	pub deployment_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetDeploymentBuildLogsRequest {
	pub start_time: Option<Interval>,
}

impl ApiRequest for GetDeploymentBuildLogsRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetDeploymentBuildLogsPath;
	type RequestQuery = Self;
	type RequestBody = ();
	type Response = GetDeploymentBuildLogsResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetDeploymentBuildLogsResponse {
	pub logs: Vec<BuildLog>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{
		GetDeploymentBuildLogsRequest,
		GetDeploymentBuildLogsResponse,
	};
	use crate::{
		models::workspace::infrastructure::deployment::{BuildLog, Interval},
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GetDeploymentBuildLogsRequest {
				start_time: Some(Interval::Hour),
			},
			&[
				Token::Struct {
					name: "GetDeploymentBuildLogsRequest",
					len: 1,
				},
				Token::Str("startTime"),
				Token::Some,
				Token::UnitVariant {
					name: "Interval",
					variant: "hour",
				},
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&GetDeploymentBuildLogsResponse {
				logs: vec![BuildLog {
					timestamp: Some(1431648000),
					reason: Some("NoPods".to_string()),
					message: Some("No matching pods found".to_string()),
				}],
			},
			&[
				Token::Struct {
					name: "GetDeploymentBuildLogsResponse",
					len: 1,
				},
				Token::Str("logs"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "BuildLog",
					len: 3,
				},
				Token::Str("timestamp"),
				Token::Some,
				Token::U64(1431648000),
				Token::Str("reason"),
				Token::Some,
				Token::Str("NoPods"),
				Token::Str("message"),
				Token::Some,
				Token::Str("No matching pods found"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GetDeploymentBuildLogsResponse {
				logs: vec![BuildLog {
					timestamp: Some(1431648000),
					reason: Some("NoPods".to_string()),
					message: Some("No matching pods found".to_string()),
				}],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("logs"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "BuildLog",
					len: 3,
				},
				Token::Str("timestamp"),
				Token::Some,
				Token::U64(1431648000),
				Token::Str("reason"),
				Token::Some,
				Token::Str("NoPods"),
				Token::Str("message"),
				Token::Some,
				Token::Str("No matching pods found"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		)
	}
}
