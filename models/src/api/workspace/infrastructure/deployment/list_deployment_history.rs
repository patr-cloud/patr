use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::DeploymentDeployHistory;
use crate::{
	utils::{Paginated, Uuid},
	ApiRequest,
};

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
#[typed_path("/workspace/:workspace_id/infrastructure/deployment/:deployment_id/deploy-history")]
pub struct ListDeploymentHistoryPath {
	pub workspace_id: Uuid,
	pub deployment_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListDeploymentHistoryRequest {}

impl ApiRequest for ListDeploymentHistoryRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListDeploymentHistoryPath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = ListDeploymentHistoryResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListDeploymentHistoryResponse {
	pub deploys: Vec<DeploymentDeployHistory>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{
		DeploymentDeployHistory,
		ListDeploymentHistoryRequest,
		ListDeploymentHistoryResponse,
	};
	use crate::ApiResponse;

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ListDeploymentHistoryRequest {},
			&[
				Token::Struct {
					name: "ListDeploymentHistoryRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&ListDeploymentHistoryResponse {
				deploys: vec![DeploymentDeployHistory{
					image_digest: "9a1343f3ee393dd9f3ae7ea5e2000ff174dbe26c6ed92e92ca350346e6a7bac3".to_string(),
					created: 6789123712,
				}],
			},
			&[
				Token::Struct {
					name: "ListDeploymentHistoryResponse",
					len: 1,
				},
				Token::Str("deploys"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "DeploymentDeployHistory",
					len: 2
				},
				Token::Str("imageDigest"),
				Token::Str("9a1343f3ee393dd9f3ae7ea5e2000ff174dbe26c6ed92e92ca350346e6a7bac3"),
				Token::Str("created"),
				Token::U64(6789123712),
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(ListDeploymentHistoryResponse {
				deploys: vec![DeploymentDeployHistory{
					image_digest: "9a1343f3ee393dd9f3ae7ea5e2000ff174dbe26c6ed92e92ca350346e6a7bac3".to_string(),
					created: 6789123712,
				}],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("deploys"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "DeploymentDeployHistory",
					len: 2
				},
				Token::Str("imageDigest"),
				Token::Str("9a1343f3ee393dd9f3ae7ea5e2000ff174dbe26c6ed92e92ca350346e6a7bac3"),
				Token::Str("created"),
				Token::U64(6789123712),
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		)
	}
}
