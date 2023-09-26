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
	"/workspace/:workspace_id/docker-registry/:repository_id/image/:digest"
)]
pub struct DeleteDockerRepositoryImagePath {
	pub workspace_id: Uuid,
	pub repository_id: Uuid,
	pub digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeleteDockerRepositoryImageRequest;

impl ApiRequest for DeleteDockerRepositoryImageRequest {
	const METHOD: Method = Method::DELETE;
	const IS_PROTECTED: bool = true;

	type RequestPath = DeleteDockerRepositoryImagePath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = ();
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::DeleteDockerRepositoryImageRequest;
	use crate::{ApiRequest, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&DeleteDockerRepositoryImageRequest,
			&[Token::UnitStruct {
				name: "DeleteDockerRepositoryImageRequest",
			}],
		);
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<
			<DeleteDockerRepositoryImageRequest as ApiRequest>::Response,
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
