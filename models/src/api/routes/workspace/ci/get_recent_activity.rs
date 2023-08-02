use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::git_provider::{BuildDetails, GitProvider, RepositoryDetails};
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
#[typed_path("/workspace/:workspace_id/ci/recent-activity")]
pub struct GetRecentActivityPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetRecentActivityRequest {}

impl ApiRequest for GetRecentActivityRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetRecentActivityPath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = GetRecentActivityResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecentActivity {
	pub git_provider_details: GitProvider,
	pub repo_details: RepositoryDetails,
	pub build_details: BuildDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GetRecentActivityResponse {
	pub activities: Vec<RecentActivity>,
}

#[cfg(test)]
mod test {
	use std::str::FromStr;

	use serde_test::{assert_tokens, Token};

	use super::{
		GetRecentActivityRequest,
		GetRecentActivityResponse,
		RecentActivity,
	};
	use crate::{
		models::workspace::ci::git_provider::{
			BuildDetails,
			BuildStatus,
			GitProvider,
			GitProviderType,
			RepoStatus,
			RepositoryDetails,
		},
		utils::{DateTime, Uuid},
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GetRecentActivityRequest {},
			&[
				Token::Struct {
					name: "GetRecentActivityRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&GetRecentActivityResponse {
				activities: vec![RecentActivity {
					git_provider_details: GitProvider {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						domain_name: "github.com".to_string(),
						git_provider_type: GitProviderType::Github,
						login_name: Some("login-name".to_string()),
						is_syncing: false,
						last_synced: None,
						is_deleted: false,
					},
					repo_details: RepositoryDetails {
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
					build_details: BuildDetails {
						build_num: 1,
						git_ref: "refs/heads/master".to_string(),
						git_commit: "e3320539a4c03ccfda992641646deb67d8bf98f3"
							.to_string(),
						git_commit_message: Some("Initial Commit".to_string()),
						git_pr_title: Some("Title".to_string()),
						author: Some("author".to_string()),
						status: BuildStatus::Errored,
						created: DateTime::from_str(
							"2020-04-12 22:10:57+02:00",
						)
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
						runner_id: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30869",
						)
						.unwrap(),
					},
				}],
			},
			&[
				Token::Struct {
					name: "GetRecentActivityResponse",
					len: 1,
				},
				Token::Str("activities"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "RecentActivity",
					len: 3,
				},
				Token::Str("gitProviderDetails"),
				Token::Struct {
					name: "GitProvider",
					len: 7,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("domainName"),
				Token::Str("github.com"),
				Token::Str("gitProviderType"),
				Token::Enum {
					name: "GitProviderType",
				},
				Token::Str("github"),
				Token::Unit,
				Token::Str("loginName"),
				Token::Some,
				Token::Str("login-name"),
				Token::Str("isSyncing"),
				Token::Bool(false),
				Token::Str("lastSynced"),
				Token::None,
				Token::Str("isDeleted"),
				Token::Bool(false),
				Token::StructEnd,
				Token::Str("repoDetails"),
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
				Token::Str("buildDetails"),
				Token::Struct {
					name: "BuildDetails",
					len: 11,
				},
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
				Token::Str("runnerId"),
				Token::Str("2aef18631ded45eb9170dc2166b30869"),
				Token::StructEnd,
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GetRecentActivityResponse {
				activities: vec![RecentActivity {
					git_provider_details: GitProvider {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						domain_name: "github.com".to_string(),
						git_provider_type: GitProviderType::Github,
						login_name: Some("login-name".to_string()),
						is_syncing: false,
						last_synced: None,
						is_deleted: false,
					},
					repo_details: RepositoryDetails {
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
					build_details: BuildDetails {
						build_num: 1,
						git_ref: "refs/heads/master".to_string(),
						git_commit: "e3320539a4c03ccfda992641646deb67d8bf98f3"
							.to_string(),
						git_commit_message: Some("Initial Commit".to_string()),
						git_pr_title: Some("Title".to_string()),
						author: Some("author".to_string()),
						status: BuildStatus::Errored,
						created: DateTime::from_str(
							"2020-04-12 22:10:57+02:00",
						)
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
						runner_id: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30869",
						)
						.unwrap(),
					},
				}],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("activities"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "RecentActivity",
					len: 3,
				},
				Token::Str("gitProviderDetails"),
				Token::Struct {
					name: "GitProvider",
					len: 7,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("domainName"),
				Token::Str("github.com"),
				Token::Str("gitProviderType"),
				Token::Enum {
					name: "GitProviderType",
				},
				Token::Str("github"),
				Token::Unit,
				Token::Str("loginName"),
				Token::Some,
				Token::Str("login-name"),
				Token::Str("isSyncing"),
				Token::Bool(false),
				Token::Str("lastSynced"),
				Token::None,
				Token::Str("isDeleted"),
				Token::Bool(false),
				Token::StructEnd,
				Token::Str("repoDetails"),
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
				Token::Str("buildDetails"),
				Token::Struct {
					name: "BuildDetails",
					len: 11,
				},
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
				Token::Str("runnerId"),
				Token::Str("2aef18631ded45eb9170dc2166b30869"),
				Token::StructEnd,
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
