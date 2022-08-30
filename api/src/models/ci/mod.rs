pub mod file_format;
pub mod webhook_payload;

pub enum EventType {
	Commit(Commit),
	Tag(Tag),
	PullRequest(PullRequest),
}

pub struct PullRequest {
	pub head_repo_owner: String,
	pub head_repo_name: String,
	pub commit_sha: String,
	pub pr_number: String,
	pub to_be_committed_branch_name: String,
}

pub struct Tag {
	pub repo_owner: String,
	pub repo_name: String,
	pub commit_sha: String,
	pub tag_name: String,
}

pub struct Commit {
	pub repo_owner: String,
	pub repo_name: String,
	pub commit_sha: String,
	pub committed_branch_name: String,
}
