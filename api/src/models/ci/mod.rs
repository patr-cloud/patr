use serde::{Deserialize, Serialize};

pub mod webhook_payload;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
	Commit(Commit),
	Tag(Tag),
	PullRequest(PullRequest),
}

impl EventType {
	pub fn repo_owner(&self) -> &str {
		match self {
			EventType::Commit(commit) => &commit.repo_owner,
			EventType::Tag(tag) => &tag.repo_owner,
			EventType::PullRequest(pr) => &pr.repo_owner,
		}
	}
	pub fn repo_name(&self) -> &str {
		match self {
			EventType::Commit(commit) => &commit.repo_name,
			EventType::Tag(tag) => &tag.repo_name,
			EventType::PullRequest(pr) => &pr.repo_name,
		}
	}
	pub fn commit_sha(&self) -> &str {
		match self {
			EventType::Commit(commit) => &commit.commit_sha,
			EventType::Tag(tag) => &tag.commit_sha,
			EventType::PullRequest(pr) => &pr.commit_sha,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
	pub pr_repo_owner: String,
	pub pr_repo_name: String,
	pub repo_owner: String,
	pub repo_name: String,
	pub commit_sha: String,
	pub pr_number: String,
	pub pr_title: String,
	pub author: String,
	pub to_be_committed_branch_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
	pub repo_owner: String,
	pub repo_name: String,
	pub commit_sha: String,
	pub tag_name: String,
	pub author: String,
	pub commit_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
	pub repo_owner: String,
	pub repo_name: String,
	pub commit_sha: String,
	pub commit_message: Option<String>,
	pub author: String,
	pub committed_branch_name: String,
}
