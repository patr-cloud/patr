use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::RepositoryDetails;
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
#[typed_path("/workspace/:workspace_id/ci/git-provider/github/repo")]
pub struct ListReposPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListReposRequest {}

impl ApiRequest for ListReposRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListReposPath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = ListReposResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListReposResponse {
	pub repos: Vec<RepositoryDetails>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{ListReposRequest, ListReposResponse, RepositoryDetails};
	use crate::{
		models::workspace::ci::git_provider::RepoStatus,
		prelude::Uuid,
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ListReposRequest {},
			&[
				Token::Struct {
					name: "ListReposRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&RepositoryDetails {
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
			&[
				Token::Struct {
					name: "RepositoryDetails",
					len: 6,
				},
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
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(ListReposResponse {
				repos: vec![RepositoryDetails {
					id: "1234".to_string(),
					name: "name".to_string(),
					repo_owner: "repo owner".to_string(),
					clone_url: "https://example.com/git_url".to_string(),
					runner_id: Some(
						Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
					),
					status: RepoStatus::Active,
				}],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("repos"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "RepositoryDetails",
					len: 6,
				},
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
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		)
	}
}
