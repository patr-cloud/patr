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
	"/workspace/:workspace_id/infrastructure/deployment/:deployment_id"
)]
pub struct DeleteDeploymentPath {
	pub workspace_id: Uuid,
	pub deployment_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeleteDeploymentRequest {
	#[serde(default)]
	/// hard_delete will be considered only if it is byoc region
	/// if hard_delete not present, then take false as default
	/// NOTE: hard_delete should be shown in frontend iff RegionStatus::Active,
	/// else the region won't get deleted due to kubeconfig error
	pub hard_delete: bool,
}

impl ApiRequest for DeleteDeploymentRequest {
	const METHOD: Method = Method::DELETE;
	const IS_PROTECTED: bool = true;

	type RequestPath = DeleteDeploymentPath;
	type RequestQuery = Self;
	type RequestBody = ();
	type Response = ();
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::DeleteDeploymentRequest;
	use crate::{ApiRequest, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&DeleteDeploymentRequest { hard_delete: false },
			&[
				Token::Struct {
					name: "DeleteDeploymentRequest",
					len: 1,
				},
				Token::Str("hardDelete"),
				Token::Bool(false),
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<<DeleteDeploymentRequest as ApiRequest>::Response>(
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
		)
	}
}
