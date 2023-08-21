use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

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
#[typed_path(
	"/workspace/:workspace_id/ci/git-provider/github/repo/:repo_id/ref"
)]
pub struct ListGitRefForRepoPath {
	pub workspace_id: Uuid,
	pub repo_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListGitRefForRepoRequest {}

impl ApiRequest for ListGitRefForRepoRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListGitRefForRepoPath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = ListGitRefForRepoResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RefType {
	Branch,
	Tag,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Ref {
	#[serde(rename = "type")]
	pub type_: RefType,
	pub name: String,
	pub latest_commit_sha: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListGitRefForRepoResponse {
	pub refs: Vec<Ref>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{
		ListGitRefForRepoRequest,
		ListGitRefForRepoResponse,
		Ref,
		RefType,
	};
	use crate::ApiResponse;

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ListGitRefForRepoRequest {},
			&[
				Token::Struct {
					name: "ListGitRefForRepoRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&ListGitRefForRepoResponse {
				refs: vec![Ref {
					type_: RefType::Branch,
					name: "name".into(),
					latest_commit_sha: "121f71f0da".into(),
				}],
			},
			&[
				Token::Struct {
					name: "ListGitRefForRepoResponse",
					len: 1,
				},
				Token::Str("refs"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "Ref",
					len: 3,
				},
				Token::Str("type"),
				Token::Enum { name: "RefType" },
				Token::Str("branch"),
				Token::Unit,
				Token::Str("name"),
				Token::Str("name"),
				Token::Str("latestCommitSha"),
				Token::Str("121f71f0da"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(ListGitRefForRepoResponse {
				refs: vec![Ref {
					type_: RefType::Branch,
					name: "name".into(),
					latest_commit_sha: "121f71f0da".into(),
				}],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("refs"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "Ref",
					len: 3,
				},
				Token::Str("type"),
				Token::Enum { name: "RefType" },
				Token::Str("branch"),
				Token::Unit,
				Token::Str("name"),
				Token::Str("name"),
				Token::Str("latestCommitSha"),
				Token::Str("121f71f0da"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
