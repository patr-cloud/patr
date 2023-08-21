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
#[typed_path("/workspace/:workspace_id")]
pub struct UpdateWorkspaceInfoPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWorkspaceInfoRequest {
	pub name: Option<String>,
	pub alert_emails: Option<Vec<String>>,
	pub default_payment_method_id: Option<String>,
}

impl ApiRequest for UpdateWorkspaceInfoRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = UpdateWorkspaceInfoPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = ();
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::UpdateWorkspaceInfoRequest;
	use crate::{ApiRequest, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&UpdateWorkspaceInfoRequest {
				name: Some("Not John Patr's Company".to_string()),
				alert_emails: Some(vec!["johnpatr@patr.cloud".to_string()]),
				default_payment_method_id: Some(
					"pm_6K95KhSGEPBh7GrIsWVB4pyV".to_string(),
				),
			},
			&[
				Token::Struct {
					name: "UpdateWorkspaceInfoRequest",
					len: 3,
				},
				Token::Str("name"),
				Token::Some,
				Token::Str("Not John Patr's Company"),
				Token::Str("alertEmails"),
				Token::Some,
				Token::Seq { len: Some(1) },
				Token::Str("johnpatr@patr.cloud"),
				Token::SeqEnd,
				Token::Str("defaultPaymentMethodId"),
				Token::Some,
				Token::Str("pm_6K95KhSGEPBh7GrIsWVB4pyV"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<
			<UpdateWorkspaceInfoRequest as ApiRequest>::Response,
		>(());
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(()),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::MapEnd,
			],
		);
	}
}
