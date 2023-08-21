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
#[typed_path("/workspace/:workspace_id/infrastructure/managed-database")]
pub struct CreateManagedDatabasePath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateManagedDatabaseRequest {
	pub name: String,
	pub db_name: String,
	pub version: String,
	pub engine: String,
	pub num_nodes: u16,
	pub database_plan: String,
	pub region: String,
}

impl ApiRequest for CreateManagedDatabaseRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = CreateManagedDatabasePath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = CreateManagedDatabaseResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateManagedDatabaseResponse {
	pub id: Uuid,
}

#[cfg(test)]
mod tests {
	use serde_test::{assert_tokens, Token};

	use super::{
		CreateManagedDatabaseRequest,
		CreateManagedDatabaseResponse,
		ManagedDatabase,
		ManagedDatabaseConnection,
	};
	use crate::{utils::Uuid, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&CreateManagedDatabaseRequest {
				name: "mydb".to_string(),
				db_name: "api".to_string(),
				version: "1.2.3".to_string(),
				engine: "mysql".to_string(),
				num_nodes: 2,
				database_plan: "free".to_string(),
				region: "bangalore".to_string(),
			},
			&[
				Token::Struct {
					name: "CreateManagedDatabaseRequest",
					len: 7,
				},
				Token::Str("name"),
				Token::Str("mydb"),
				Token::Str("dbName"),
				Token::Str("api"),
				Token::Str("version"),
				Token::Str("1.2.3"),
				Token::Str("engine"),
				Token::Str("mysql"),
				Token::Str("num_nodes"),
				Token::U16(2),
				Token::Str("databasePlan"),
				Token::Str("free"),
				Token::Str("region"),
				Token::Str("bangalore"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&CreateManagedDatabaseResponse {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
			},
			&[
				Token::Struct {
					name: "CreateManagedDatabaseResponse",
					len: 1,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(CreateManagedDatabaseResponse {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Struct {
					name: "CreateManagedDatabaseResponse",
					len: 1,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::StructEnd,
				Token::MapEnd,
			],
		);
	}
}
