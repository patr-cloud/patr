use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::{BuildDetails, Step};
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
#[typed_path("/workspace/:workspace_id/ci/git-provider/github/repo/:repo_id/build/:build_num")]
pub struct GetBuildInfoPath {
	pub workspace_id: Uuid,
	pub repo_id: String,
	pub build_num: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetBuildInfoRequest {}

impl ApiRequest for GetBuildInfoRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetBuildInfoPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = GetBuildInfoResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetBuildInfoResponse {
	#[serde(flatten)]
	pub build_info: BuildDetails,
	pub steps: Vec<Step>,
}

#[cfg(test)]
mod test {
	use std::str::FromStr;

	use serde_test::{assert_tokens, Token};

	use super::{GetBuildInfoRequest, GetBuildInfoResponse};
	use crate::{
		models::workspace::ci::git_provider::{
			BuildDetails,
			BuildStatus,
			BuildStepStatus,
			Step,
		},
		prelude::Uuid,
		utils::DateTime,
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GetBuildInfoRequest {},
			&[
				Token::Struct {
					name: "GetBuildInfoRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&GetBuildInfoResponse {
				build_info: BuildDetails {
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
				},
				steps: vec![Step {
					step_id: 1,
					step_name: "one".to_string(),
					base_image: "alpine/alpine".to_string(),
					commands: "echo hello world".to_string(),
					status: BuildStepStatus::Succeeded,
					started: Some(
						DateTime::from_str("2020-04-12 22:11:57+02:00")
							.unwrap(),
					),
					finished: Some(
						DateTime::from_str("2020-04-12 22:15:57+02:00")
							.unwrap(),
					),
				}],
			},
			&[
				Token::Map { len: None },
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
				Token::Str("steps"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "Step",
					len: 7,
				},
				Token::Str("stepId"),
				Token::U32(1),
				Token::Str("stepName"),
				Token::Str("one"),
				Token::Str("baseImage"),
				Token::Str("alpine/alpine"),
				Token::Str("commands"),
				Token::Str("echo hello world"),
				Token::Str("status"),
				Token::UnitVariant {
					name: "BuildStepStatus",
					variant: "succeeded",
				},
				Token::Str("started"),
				Token::Some,
				Token::Str("Sun, 12 Apr 2020 20:11:57 +0000"),
				Token::Str("finished"),
				Token::Some,
				Token::Str("Sun, 12 Apr 2020 20:15:57 +0000"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GetBuildInfoResponse {
				build_info: BuildDetails {
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
				},
				steps: vec![Step {
					step_id: 1,
					step_name: "one".to_string(),
					base_image: "alpine/alpine".to_string(),
					commands: "echo hello world".to_string(),
					status: BuildStepStatus::Succeeded,
					started: Some(
						DateTime::from_str("2020-04-12 22:11:57+02:00")
							.unwrap(),
					),
					finished: Some(
						DateTime::from_str("2020-04-12 22:15:57+02:00")
							.unwrap(),
					),
				}],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
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
				Token::Str("steps"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "Step",
					len: 7,
				},
				Token::Str("stepId"),
				Token::U32(1),
				Token::Str("stepName"),
				Token::Str("one"),
				Token::Str("baseImage"),
				Token::Str("alpine/alpine"),
				Token::Str("commands"),
				Token::Str("echo hello world"),
				Token::Str("status"),
				Token::UnitVariant {
					name: "BuildStepStatus",
					variant: "succeeded",
				},
				Token::Str("started"),
				Token::Some,
				Token::Str("Sun, 12 Apr 2020 20:11:57 +0000"),
				Token::Str("finished"),
				Token::Some,
				Token::Str("Sun, 12 Apr 2020 20:15:57 +0000"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
