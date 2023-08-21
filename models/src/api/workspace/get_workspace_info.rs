use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::Workspace;
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
#[typed_path("/workspace/:workspace_id/info")]
pub struct GetWorkspaceInfoPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetWorkspaceInfoRequest;

impl ApiRequest for GetWorkspaceInfoRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetWorkspaceInfoPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = GetWorkspaceInfoResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetWorkspaceInfoResponse {
	#[serde(flatten)]
	pub workspace: Workspace,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{GetWorkspaceInfoRequest, GetWorkspaceInfoResponse};
	use crate::{models::workspace::Workspace, utils::Uuid, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GetWorkspaceInfoRequest,
			&[Token::UnitStruct {
				name: "GetWorkspaceInfoRequest",
			}],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&GetWorkspaceInfoResponse {
				workspace: Workspace {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					name: "John Patr's Company".to_string(),
					super_admin_id: Uuid::parse_str(
						"2aef18631ded45eb9170dc2166b30898",
					)
					.unwrap(),
					active: true,
					alert_emails: vec!["johnpatr@patr.com".to_string()],
					default_payment_method_id: Some(
						"pm_6K95KhSGEPBh7GrIsWVB4pyV".to_string(),
					),
					is_verified: true,
				},
			},
			&[
				Token::Map { len: None },
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("John Patr's Company"),
				Token::Str("superAdminId"),
				Token::Str("2aef18631ded45eb9170dc2166b30898"),
				Token::Str("active"),
				Token::Bool(true),
				Token::Str("alertEmails"),
				Token::Seq { len: Some(1) },
				Token::Str("johnpatr@patr.com"),
				Token::SeqEnd,
				Token::Str("defaultPaymentMethodId"),
				Token::Some,
				Token::Str("pm_6K95KhSGEPBh7GrIsWVB4pyV"),
				Token::Str("isVerified"),
				Token::Bool(true),
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GetWorkspaceInfoResponse {
				workspace: Workspace {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					name: "John Patr's Company".to_string(),
					super_admin_id: Uuid::parse_str(
						"2aef18631ded45eb9170dc2166b30898",
					)
					.unwrap(),
					active: true,
					alert_emails: vec!["johnpatr@patr.com".to_string()],
					default_payment_method_id: Some(
						"pm_6K95KhSGEPBh7GrIsWVB4pyV".to_string(),
					),
					is_verified: false,
				},
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("John Patr's Company"),
				Token::Str("superAdminId"),
				Token::Str("2aef18631ded45eb9170dc2166b30898"),
				Token::Str("active"),
				Token::Bool(true),
				Token::Str("alertEmails"),
				Token::Seq { len: Some(1) },
				Token::Str("johnpatr@patr.com"),
				Token::SeqEnd,
				Token::Str("defaultPaymentMethodId"),
				Token::Some,
				Token::Str("pm_6K95KhSGEPBh7GrIsWVB4pyV"),
				Token::Str("isVerified"),
				Token::Bool(false),
				Token::MapEnd,
			],
		);
	}
}
