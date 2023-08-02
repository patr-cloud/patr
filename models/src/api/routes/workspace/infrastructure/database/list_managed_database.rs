use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::ManagedDatabase;
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
pub struct ListAllManagedDatabasePath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListAllManagedDatabaseRequest {}

impl ApiRequest for ListAllManagedDatabaseRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListAllManagedDatabasePath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = ListAllManagedDatabaseResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListAllManagedDatabaseResponse {
	pub databases: Vec<ManagedDatabase>,
}

#[cfg(test)]
mod tests {
	use serde_test::{assert_tokens, Token};

	use super::{
		ListAllManagedDatabaseRequest,
		ListAllManagedDatabaseResponse,
		ManagedDatabase,
		ManagedDatabaseConnection,
	};
	use crate::{utils::Uuid, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ListAllManagedDatabaseRequest {},
			&[
				Token::Struct {
					name: "ListAllManagedDatabaseRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&ListAllManagedDatabaseResponse {
				databases: vec![ManagedDatabase {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					name: "mydb".to_string(),
					database_name: "api".to_string(),
					engine: "mysql".to_string(),
					version: "1.2.3".to_string(),
					num_nodes: 2,
					database_plan: "free".to_string(),
					region: "bangalore".to_string(),
					status: "running".to_string(),
					public_connection: ManagedDatabaseConnection {
						host: "host".to_string(),
						port: 5678,
						username: "username".to_string(),
						password: "password".to_string(),
					},
				}],
			},
			&[
				Token::Struct {
					name: "ListAllManagedDatabaseResponse",
					len: 1,
				},
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "ManagedDatabase",
					len: 10,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("mydb"),
				Token::Str("databaseName"),
				Token::Str("api"),
				Token::Str("engine"),
				Token::Str("mysql"),
				Token::Str("version"),
				Token::Str("1.2.3"),
				Token::Str("numNodes"),
				Token::U16(2),
				Token::Str("databasePlan"),
				Token::Str("free"),
				Token::Str("region"),
				Token::Str("bangalore"),
				Token::Str("status"),
				Token::Str("running"),
				Token::Str("publicConnection"),
				Token::Struct {
					name: "ManagedDatabaseConnection",
					len: 4,
				},
				Token::Str("host"),
				Token::Str("host"),
				Token::Str("port"),
				Token::U32(5678),
				Token::Str("username"),
				Token::Str("username"),
				Token::Str("password"),
				Token::Str("password"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(ListAllManagedDatabaseResponse {
				databases: vec![ManagedDatabase {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					name: "mydb".to_string(),
					database_name: "api".to_string(),
					engine: "mysql".to_string(),
					version: "1.2.3".to_string(),
					num_nodes: 2,
					database_plan: "free".to_string(),
					region: "bangalore".to_string(),
					status: "running".to_string(),
					public_connection: ManagedDatabaseConnection {
						host: "host".to_string(),
						port: 5678,
						username: "username".to_string(),
						password: "password".to_string(),
					},
				}],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Struct {
					name: "ListAllManagedDatabaseResponse",
					len: 10,
				},
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "ManagedDatabase",
					len: 10,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("mydb"),
				Token::Str("databaseName"),
				Token::Str("api"),
				Token::Str("engine"),
				Token::Str("mysql"),
				Token::Str("version"),
				Token::Str("1.2.3"),
				Token::Str("numNodes"),
				Token::U16(2),
				Token::Str("databasePlan"),
				Token::Str("free"),
				Token::Str("region"),
				Token::Str("bangalore"),
				Token::Str("status"),
				Token::Str("running"),
				Token::Str("publicConnection"),
				Token::Struct {
					name: "ManagedDatabaseConnection",
					len: 4,
				},
				Token::Str("host"),
				Token::Str("host"),
				Token::Str("port"),
				Token::U32(5678),
				Token::Str("username"),
				Token::Str("username"),
				Token::Str("password"),
				Token::Str("password"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
				Token::MapEnd,
			],
		);
	}
}
