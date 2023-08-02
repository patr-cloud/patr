use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::{models::workspace::Workspace, utils::Paginated, ApiRequest};

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
#[typed_path("/user/workspaces")]
pub struct ListUserWorkspacesPath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListUserWorkspacesRequest;

impl ApiRequest for ListUserWorkspacesRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListUserWorkspacesPath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = ListUserWorkspacesResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListUserWorkspacesResponse {
	pub workspaces: Vec<Workspace>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{ListUserWorkspacesRequest, ListUserWorkspacesResponse};
	use crate::{models::workspace::Workspace, utils::Uuid, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ListUserWorkspacesRequest,
			&[Token::UnitStruct {
				name: "ListUserWorkspacesRequest",
			}],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&ListUserWorkspacesResponse {
				workspaces: vec![
					Workspace {
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
					Workspace {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30868")
							.unwrap(),
						super_admin_id: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30899",
						)
						.unwrap(),
						name: "Not John Patr's Company".to_string(),
						active: true,
						alert_emails: vec!["johnpatr@patr2.com".to_string()],
						default_payment_method_id: None,
						is_verified: false,
					},
				],
			},
			&[
				Token::Struct {
					name: "ListUserWorkspacesResponse",
					len: 1,
				},
				Token::Str("workspaces"),
				Token::Seq { len: Some(2) },
				Token::Struct {
					name: "Workspace",
					len: 7,
				},
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
				Token::StructEnd,
				Token::Struct {
					name: "Workspace",
					len: 7,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Str("name"),
				Token::Str("Not John Patr's Company"),
				Token::Str("superAdminId"),
				Token::Str("2aef18631ded45eb9170dc2166b30899"),
				Token::Str("active"),
				Token::Bool(true),
				Token::Str("alertEmails"),
				Token::Seq { len: Some(1) },
				Token::Str("johnpatr@patr2.com"),
				Token::SeqEnd,
				Token::Str("defaultPaymentMethodId"),
				Token::None,
				Token::Str("isVerified"),
				Token::Bool(false),
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(ListUserWorkspacesResponse {
				workspaces: vec![
					Workspace {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						name: "John Patr's Company".to_string(),
						super_admin_id: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30898",
						)
						.unwrap(),
						active: true,
						alert_emails: vec!["johnpatr@patr.com".to_string()],
						default_payment_method_id: None,
						is_verified: true,
					},
					Workspace {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30868")
							.unwrap(),
						name: "Not John Patr's Company".to_string(),
						super_admin_id: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30899",
						)
						.unwrap(),
						active: true,
						alert_emails: vec!["johnpatr@patr2.com".to_string()],
						default_payment_method_id: None,
						is_verified: false,
					},
				],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("workspaces"),
				Token::Seq { len: Some(2) },
				Token::Struct {
					name: "Workspace",
					len: 7,
				},
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
				Token::None,
				Token::Str("isVerified"),
				Token::Bool(true),
				Token::StructEnd,
				Token::Struct {
					name: "Workspace",
					len: 7,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Str("name"),
				Token::Str("Not John Patr's Company"),
				Token::Str("superAdminId"),
				Token::Str("2aef18631ded45eb9170dc2166b30899"),
				Token::Str("active"),
				Token::Bool(true),
				Token::Str("alertEmails"),
				Token::Seq { len: Some(1) },
				Token::Str("johnpatr@patr2.com"),
				Token::SeqEnd,
				Token::Str("defaultPaymentMethodId"),
				Token::None,
				Token::Str("isVerified"),
				Token::Bool(false),
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
