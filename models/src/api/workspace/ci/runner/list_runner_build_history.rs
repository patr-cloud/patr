use axum_extra::routing::TypedPath;
use chrono::Utc;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::{
	models::workspace::ci::git_provider::BuildStatus,
	utils::{DateTime, Paginated, Uuid},
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
#[typed_path("/workspace/:workspace_id/ci/runner/:runner_id/history")]
pub struct ListCiRunnerBuildHistoryPath {
	pub workspace_id: Uuid,
	pub runner_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListCiRunnerBuildHistoryRequest {}

impl ApiRequest for ListCiRunnerBuildHistoryRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListCiRunnerBuildHistoryPath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = ListCiRunnerBuildHistoryResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunnerBuildDetails {
	// todo: when supporting multiple provider need to add git_provider field
	// pub git_provider: Uuid,
	pub github_repo_id: String,
	pub build_num: u64,
	pub git_ref: String,
	pub git_commit: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub git_commit_message: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub git_pr_title: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub author: Option<String>,
	pub status: BuildStatus,
	pub created: DateTime<Utc>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub started: Option<DateTime<Utc>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub finished: Option<DateTime<Utc>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListCiRunnerBuildHistoryResponse {
	pub builds: Vec<RunnerBuildDetails>,
}

#[cfg(test)]
mod test {
	use std::str::FromStr;

	use serde_test::{assert_tokens, Token};

	use super::{
		ListCiRunnerBuildHistoryRequest,
		ListCiRunnerBuildHistoryResponse,
		RunnerBuildDetails,
	};
	use crate::{
		models::workspace::ci::git_provider::BuildStatus,
		utils::DateTime,
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ListCiRunnerBuildHistoryRequest {},
			&[
				Token::Struct {
					name: "ListCiRunnerBuildHistoryRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&ListCiRunnerBuildHistoryResponse {
				builds: vec![RunnerBuildDetails {
					github_repo_id: "repo-id".into(),
					build_num: 1,
					git_ref: "refs/heads/master".to_string(),
					git_commit: "e3320539a4c03ccfda992641646deb67d8bf98f3"
						.to_string(),
					git_commit_message: Some("Initial Commit".to_string()),
					git_pr_title: Some("Title".to_string()),
					author: Some("author".to_string()),
					status: BuildStatus::Errored,
					created: DateTime::from_str("2020-04-12 22:10:57+02:00")
						.unwrap(),
					started: Some(
						DateTime::from_str("2020-04-12 22:16:57+02:00")
							.unwrap(),
					),
					finished: Some(
						DateTime::from_str("2020-04-12 22:16:57+02:00")
							.unwrap(),
					),
					message: None,
				}],
			},
			&[
				Token::Struct {
					name: "ListCiRunnerBuildHistoryResponse",
					len: 1,
				},
				Token::Str("builds"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "RunnerBuildDetails",
					len: 11,
				},
				Token::Str("githubRepoId"),
				Token::Str("repo-id"),
				Token::Str("buildNum"),
				Token::U64(1),
				Token::Str("gitRef"),
				Token::Str("refs/heads/master"),
				Token::Str("gitCommit"),
				Token::Str("e3320539a4c03ccfda992641646deb67d8bf98f3"),
				Token::Str("gitCommitMessage"),
				Token::Some,
				Token::Str("Initial Commit"),
				Token::Str("gitPrTitle"),
				Token::Some,
				Token::Str("Title"),
				Token::Str("author"),
				Token::Some,
				Token::Str("author"),
				Token::Str("status"),
				Token::UnitVariant {
					name: "BuildStatus",
					variant: "errored",
				},
				Token::Str("created"),
				Token::Str("Sun, 12 Apr 2020 20:10:57 +0000"),
				Token::Str("started"),
				Token::Some,
				Token::Str("Sun, 12 Apr 2020 20:16:57 +0000"),
				Token::Str("finished"),
				Token::Some,
				Token::Str("Sun, 12 Apr 2020 20:16:57 +0000"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(ListCiRunnerBuildHistoryResponse {
				builds: vec![RunnerBuildDetails {
					github_repo_id: "repo-id".into(),
					build_num: 1,
					git_ref: "refs/heads/master".to_string(),
					git_commit: "e3320539a4c03ccfda992641646deb67d8bf98f3"
						.to_string(),
					git_commit_message: Some("Initial Commit".to_string()),
					git_pr_title: Some("Title".to_string()),
					author: Some("author".to_string()),
					status: BuildStatus::Errored,
					created: DateTime::from_str("2020-04-12 22:10:57+02:00")
						.unwrap(),
					started: Some(
						DateTime::from_str("2020-04-12 22:16:57+02:00")
							.unwrap(),
					),
					finished: Some(
						DateTime::from_str("2020-04-12 22:16:57+02:00")
							.unwrap(),
					),
					message: None,
				}],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("builds"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "RunnerBuildDetails",
					len: 11,
				},
				Token::Str("githubRepoId"),
				Token::Str("repo-id"),
				Token::Str("buildNum"),
				Token::U64(1),
				Token::Str("gitRef"),
				Token::Str("refs/heads/master"),
				Token::Str("gitCommit"),
				Token::Str("e3320539a4c03ccfda992641646deb67d8bf98f3"),
				Token::Str("gitCommitMessage"),
				Token::Some,
				Token::Str("Initial Commit"),
				Token::Str("gitPrTitle"),
				Token::Some,
				Token::Str("Title"),
				Token::Str("author"),
				Token::Some,
				Token::Str("author"),
				Token::Str("status"),
				Token::UnitVariant {
					name: "BuildStatus",
					variant: "errored",
				},
				Token::Str("created"),
				Token::Str("Sun, 12 Apr 2020 20:10:57 +0000"),
				Token::Str("started"),
				Token::Some,
				Token::Str("Sun, 12 Apr 2020 20:16:57 +0000"),
				Token::Str("finished"),
				Token::Some,
				Token::Str("Sun, 12 Apr 2020 20:16:57 +0000"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
