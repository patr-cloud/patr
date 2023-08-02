use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::ManagedUrlType;
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
	"/workspace/:workspace_id/infrastructure/managed-url/:managed_url_id"
)]
pub struct UpdateManagedUrlPath {
	pub workspace_id: Uuid,
	pub managed_url_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateManagedUrlRequest {
	pub path: String,
	#[serde(flatten)]
	pub url_type: ManagedUrlType,
}

impl ApiRequest for UpdateManagedUrlRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = UpdateManagedUrlPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = ();
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::UpdateManagedUrlRequest;
	use crate::{
		models::workspace::infrastructure::managed_urls::ManagedUrlType,
		utils::Uuid,
		ApiRequest,
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&UpdateManagedUrlRequest {
				path: "/".to_string(),
				url_type: ManagedUrlType::ProxyDeployment {
					deployment_id: Uuid::parse_str(
						"2aef18631ded45eb9170dc2166b30867",
					)
					.unwrap(),
					port: 8080,
				},
			},
			&[
				Token::Map { len: None },
				Token::Str("path"),
				Token::Str("/"),
				Token::Str("type"),
				Token::Str("proxyDeployment"),
				Token::Str("deploymentId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("port"),
				Token::U16(8080),
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<<UpdateManagedUrlRequest as ApiRequest>::Response>(
			(),
		);
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
