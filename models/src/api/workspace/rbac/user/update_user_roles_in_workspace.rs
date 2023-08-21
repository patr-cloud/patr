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
#[typed_path("/workspace/:workspace_id/rbac/user/:user_id")]
pub struct UpdateUserRolesInWorkspacePath {
	pub workspace_id: Uuid,
	pub user_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserRolesInWorkspaceRequest {
	pub roles: Vec<Uuid>,
}

impl ApiRequest for UpdateUserRolesInWorkspaceRequest {
	const METHOD: Method = Method::PUT;
	const IS_PROTECTED: bool = true;

	type RequestPath = UpdateUserRolesInWorkspacePath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = ();
}

#[cfg(test)]
mod test {

	use serde_test::{assert_tokens, Token};

	use super::UpdateUserRolesInWorkspaceRequest;
	use crate::{utils::Uuid, ApiRequest, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&UpdateUserRolesInWorkspaceRequest {
				roles: vec![
					Uuid::parse_str("2aef1a631ded45eb9170dc2166b30867")
						.unwrap(),
					Uuid::parse_str("2aef1c631ded45eb9170dc2166b30867")
						.unwrap(),
				],
			},
			&[
				Token::Struct {
					name: "UpdateUserRolesInWorkspaceRequest",
					len: 1,
				},
				Token::Str("roles"),
				Token::Seq { len: Some(2) },
				Token::Str("2aef1a631ded45eb9170dc2166b30867"),
				Token::Str("2aef1c631ded45eb9170dc2166b30867"),
				Token::SeqEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<
			<UpdateUserRolesInWorkspaceRequest as ApiRequest>::Response,
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
