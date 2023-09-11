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
#[typed_path(
	"/workspace/:workspace_id/infrastructure/managed-database/:database_id"
)]
pub struct DeleteManagedDatabasePath {
	pub workspace_id: Uuid,
	pub database_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeleteManagedDatabaseRequest {}

impl ApiRequest for DeleteManagedDatabaseRequest {
	const METHOD: Method = Method::DELETE;
	const IS_PROTECTED: bool = true;

	type RequestPath = DeleteManagedDatabasePath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = ();
}

#[cfg(test)]
mod tests {
	use serde_test::{assert_tokens, Token};

	use super::{
		DeleteManagedDatabaseRequest,
		ListAllManagedDatabaseResponse,
		ManagedDatabase,
		ManagedDatabaseConnection,
	};
	use crate::{utils::Uuid, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&DeleteManagedDatabaseRequest {},
			&[
				Token::Struct {
					name: "DeleteManagedDatabaseRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&ListAllManagedDatabaseResponse {},
			&[
				Token::Struct {
					name: "GetManagedDatabaseResponse",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(ListAllManagedDatabaseResponse {}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Struct {
					name: "GetManagedDatabaseResponse",
					len: 0,
				},
				Token::StructEnd,
				Token::MapEnd,
			],
		);
	}
}
