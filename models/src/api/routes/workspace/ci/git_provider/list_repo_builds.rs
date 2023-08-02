use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::BuildDetails;
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
	"/workspace/:workspace_id/ci/git-provider/github/repo/:repo_id/build"
)]
pub struct GetBuildListPath {
	pub workspace_id: Uuid,
	pub repo_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetBuildListRequest {}

impl ApiRequest for GetBuildListRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetBuildListPath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = GetBuildListResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GetBuildListResponse {
	pub builds: Vec<BuildDetails>,
}

#[cfg(test)]
mod test {
	use std::str::FromStr;

	use serde_test::{assert_tokens, Token};

	use super::{GetBuildListRequest, GetBuildListResponse};
	use crate::{
		models::workspace::ci::git_provider::{BuildDetails, BuildStatus},
		utils::{DateTime, Uuid},
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GetBuildListRequest {},
			&[
				Token::Struct {
					name: "GetBuildListRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&GetBuildListResponse {
				builds: vec![BuildDetails {
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
					runner_id: Uuid::parse_str(
						"2aef18631ded45eb9170dc2166b30869",
					)
					.unwrap(),
				}],
			},
			&[
				Token::Struct {
					name: "GetBuildListResponse",
					len: 1,
				},
				Token::Str("builds"),
				Token::Seq { len: Some(1) },
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
				Token::SeqEnd,
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GetBuildListResponse {
				builds: vec![BuildDetails {
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
					runner_id: Uuid::parse_str(
						"2aef18631ded45eb9170dc2166b30869",
					)
					.unwrap(),
				}],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("builds"),
				Token::Seq { len: Some(1) },
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
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
