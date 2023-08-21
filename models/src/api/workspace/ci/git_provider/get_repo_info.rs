use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::RepositoryDetails;
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
#[typed_path("/workspace/:workspace_id/ci/git-provider/github/repo/:repo_id")]
pub struct GetRepoInfoPath {
	pub workspace_id: Uuid,
	pub repo_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetRepoInfoRequest {}

impl ApiRequest for GetRepoInfoRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetRepoInfoPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = GetRepoInfoResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetRepoInfoResponse {
	#[serde(flatten)]
	pub repo: RepositoryDetails,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{GetRepoInfoRequest, GetRepoInfoResponse, RepositoryDetails};
	use crate::{
		models::workspace::ci::git_provider::RepoStatus,
		utils::Uuid,
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GetRepoInfoRequest {},
			&[
				Token::Struct {
					name: "GetRepoInfoRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GetRepoInfoResponse {
				repo: RepositoryDetails {
					id: "1234".to_string(),
					name: "name".to_string(),
					repo_owner: "repo owner".to_string(),
					clone_url: "https://example.com/git_url".to_string(),
					runner_id: Some(
						Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
					),
					status: RepoStatus::Active,
				},
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("id"),
				Token::Str("1234"),
				Token::Str("name"),
				Token::Str("name"),
				Token::Str("repoOwner"),
				Token::Str("repo owner"),
				Token::Str("cloneUrl"),
				Token::Str("https://example.com/git_url"),
				Token::Str("status"),
				Token::UnitVariant {
					name: "RepoStatus",
					variant: "active",
				},
				Token::Str("runnerId"),
				Token::Some,
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::MapEnd,
			],
		)
	}
}
