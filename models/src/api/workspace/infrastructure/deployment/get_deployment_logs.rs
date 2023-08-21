use axum_extra::routing::TypedPath;
use chrono::Utc;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::DeploymentLogs;
use crate::{
	utils::{DateTime, Uuid},
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
#[typed_path(
	"/workspace/:workspace_id/infrastructure/deployment/:deployment_id/logs"
)]
pub struct GetDeploymentLogsPath {
	pub workspace_id: Uuid,
	pub deployment_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetDeploymentLogsRequest {
	pub end_time: Option<DateTime<Utc>>,
	pub limit: Option<u32>,
}

impl ApiRequest for GetDeploymentLogsRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetDeploymentLogsPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = GetDeploymentLogsResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetDeploymentLogsResponse {
	pub logs: Vec<DeploymentLogs>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{GetDeploymentLogsRequest, GetDeploymentLogsResponse};
	use crate::{
		models::workspace::infrastructure::deployment::DeploymentLogs,
		utils::DateTime,
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GetDeploymentLogsRequest {
				end_time: Some(DateTime::default()),
				limit: Some(100),
			},
			&[
				Token::Struct {
					name: "GetDeploymentLogsRequest",
					len: 2,
				},
				Token::Str("endTime"),
				Token::Some,
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("limit"),
				Token::Some,
				Token::U32(100),
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&GetDeploymentLogsResponse {
				logs: vec![DeploymentLogs {
					timestamp: DateTime::default(),
					logs: "<DEPLOYMENT LOGS>".to_string(),
				}],
			},
			&[
				Token::Struct {
					name: "GetDeploymentLogsResponse",
					len: 1,
				},
				Token::Str("logs"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "DeploymentLogs",
					len: 2,
				},
				Token::Str("timestamp"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("logs"),
				Token::Str("<DEPLOYMENT LOGS>"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GetDeploymentLogsResponse {
				logs: vec![DeploymentLogs {
					timestamp: DateTime::default(),
					logs: "<DEPLOYMENT LOGS>".to_string(),
				}],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("logs"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "DeploymentLogs",
					len: 2,
				},
				Token::Str("timestamp"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("logs"),
				Token::Str("<DEPLOYMENT LOGS>"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		)
	}
}
