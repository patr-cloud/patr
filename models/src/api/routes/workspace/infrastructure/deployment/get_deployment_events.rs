use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::{
	models::workspace::WorkspaceAuditLog,
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
#[typed_path(
	"/workspace/:workspace_id/infrastructure/deployment/:deployment_id/events"
)]
pub struct GetDeploymentEventsPath {
	pub workspace_id: Uuid,
	pub deployment_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetDeploymentEventsRequest {}

impl ApiRequest for GetDeploymentEventsRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetDeploymentEventsPath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = GetDeploymentEventsResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetDeploymentEventsResponse {
	pub logs: Vec<WorkspaceAuditLog>,
}

#[cfg(test)]
mod test {
	use chrono::{TimeZone, Utc};
	use serde_test::{assert_tokens, Token};

	use super::{GetDeploymentEventsRequest, GetDeploymentEventsResponse};
	use crate::{
		models::workspace::WorkspaceAuditLog,
		utils::Uuid,
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GetDeploymentEventsRequest {},
			&[
				Token::Struct {
					name: "GetDeploymentEventsRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&GetDeploymentEventsResponse {
				logs: vec![WorkspaceAuditLog {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					date: Utc.timestamp_opt(1431648000, 0).unwrap().into(),
					ip_address: "0.0.0.0".to_string(),
					workspace_id: Uuid::parse_str(
						"2aef18631ded45eb9170dc2166b30877",
					)
					.unwrap(),
					user_id: Some(
						Uuid::parse_str("2bef18631ded45eb9170dc2166b30867")
							.unwrap(),
					),
					login_id: Some(
						Uuid::parse_str("39ef702abe1348e4a5ac1400cdc4c0b6")
							.unwrap(),
					),
					resource_id: Uuid::parse_str(
						"6567744e7dc14427a1ae6761e8db9876",
					)
					.unwrap(),
					action: "deployment::create".to_string(),
					request_id: Uuid::parse_str(
						"6567744e7dc15427a1ae6761e8db9876",
					)
					.unwrap(),
					metadata: serde_json::json!({}),
					patr_action: true,
					request_success: false,
				}],
			},
			&[
				Token::Struct {
					name: "GetDeploymentEventsResponse",
					len: 1,
				},
				Token::Str("logs"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "WorkspaceAuditLog",
					len: 12,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("date"),
				Token::Str("Fri, 15 May 2015 00:00:00 +0000"),
				Token::Str("ipAddress"),
				Token::Str("0.0.0.0"),
				Token::Str("workspaceId"),
				Token::Str("2aef18631ded45eb9170dc2166b30877"),
				Token::Str("userId"),
				Token::Some,
				Token::Str("2bef18631ded45eb9170dc2166b30867"),
				Token::Str("loginId"),
				Token::Some,
				Token::Str("39ef702abe1348e4a5ac1400cdc4c0b6"),
				Token::Str("resourceId"),
				Token::Str("6567744e7dc14427a1ae6761e8db9876"),
				Token::Str("action"),
				Token::Str("deployment::create"),
				Token::Str("requestId"),
				Token::Str("6567744e7dc15427a1ae6761e8db9876"),
				Token::Str("metadata"),
				Token::Map { len: Some(0) },
				Token::MapEnd,
				Token::Str("patrAction"),
				Token::Bool(true),
				Token::Str("requestSuccess"),
				Token::Bool(false),
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GetDeploymentEventsResponse {
				logs: vec![WorkspaceAuditLog {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					date: Utc.timestamp_opt(1431648000, 0).unwrap().into(),
					ip_address: "0.0.0.0".to_string(),
					workspace_id: Uuid::parse_str(
						"2aef18631ded45eb9170dc2166b30877",
					)
					.unwrap(),
					user_id: Some(
						Uuid::parse_str("2bef18631ded45eb9170dc2166b30867")
							.unwrap(),
					),
					login_id: Some(
						Uuid::parse_str("39ef702abe1348e4a5ac1400cdc4c0b6")
							.unwrap(),
					),
					resource_id: Uuid::parse_str(
						"6567744e7dc14427a1ae6761e8db9876",
					)
					.unwrap(),
					action: "deployment::create".to_string(),
					request_id: Uuid::parse_str(
						"6567744e7dc15427a1ae6761e8db9876",
					)
					.unwrap(),
					metadata: serde_json::json!({}),
					patr_action: true,
					request_success: false,
				}],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("logs"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "WorkspaceAuditLog",
					len: 12,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("date"),
				Token::Str("Fri, 15 May 2015 00:00:00 +0000"),
				Token::Str("ipAddress"),
				Token::Str("0.0.0.0"),
				Token::Str("workspaceId"),
				Token::Str("2aef18631ded45eb9170dc2166b30877"),
				Token::Str("userId"),
				Token::Some,
				Token::Str("2bef18631ded45eb9170dc2166b30867"),
				Token::Str("loginId"),
				Token::Some,
				Token::Str("39ef702abe1348e4a5ac1400cdc4c0b6"),
				Token::Str("resourceId"),
				Token::Str("6567744e7dc14427a1ae6761e8db9876"),
				Token::Str("action"),
				Token::Str("deployment::create"),
				Token::Str("requestId"),
				Token::Str("6567744e7dc15427a1ae6761e8db9876"),
				Token::Str("metadata"),
				Token::Map { len: Some(0) },
				Token::MapEnd,
				Token::Str("patrAction"),
				Token::Bool(true),
				Token::Str("requestSuccess"),
				Token::Bool(false),
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		)
	}
}
