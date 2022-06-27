use eve_rs::{AsError, Context};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::{
	service,
	utils::{Error, EveContext},
};

type HmacSha256 = Hmac<Sha256>;

pub const HUB_SIGNATURE_256: &str = "x-hub-signature-256";

/// Returns error if payload signature is different from header signature
pub async fn verify_payload_signature(
	signature_from_header: &str,
	payload: &impl AsRef<[u8]>,
	configured_secret: &impl AsRef<[u8]>,
) -> Result<(), Error> {
	// strip the sha info in header prefix
	let x_hub_signature = match signature_from_header.strip_prefix("sha256=") {
		Some(sign) => sign,
		None => signature_from_header,
	};
	let x_hub_signature = hex::decode(x_hub_signature)?;

	// calculate the sha for payload data
	let mut payload_signature =
		HmacSha256::new_from_slice(configured_secret.as_ref())?;
	payload_signature.update(payload.as_ref());

	// verify the payload sign with header sign
	payload_signature.verify_slice(&x_hub_signature)?;

	Ok(())
}

pub async fn ci_push_event(context: &mut EveContext) -> Result<(), Error> {
	// verify signature
	verify_payload_signature(
		&context
			.get_header(HUB_SIGNATURE_256)
			.status(400)
			.body("x-hub-signature-256 header not found")?,
		&context.get_request().get_body_bytes(),
		&"secret", // TODO: handle secret for each repo/user
	)
	.await?;

	let push_event = context.get_body_as::<webhook_payload::PushEvent>()?;
	let (owner_name, repo_name) =
		push_event.repository.full_name.split_once('/').unwrap();
	let repo_clone_url = push_event.repository.clone_url;
	let branch_name = push_event
		.ref_
		.strip_prefix("refs/heads/")
		.status(500)
		.body("currently only push on branches is supported")?;

	let github_client = octorust::Client::new("patr", None).unwrap(); // TODO: use github credentials for private repos
	let ci_file = github_client
		.repos()
		.get_content_file(owner_name, repo_name, "patr.yml", branch_name)
		.await
		.ok()
		.status(500)
		.body("patr.yml file is not defined")?;

	// TODO: use github credentials for private repo
	let ci_file = reqwest::get(ci_file.download_url).await?.bytes().await?;

	let config = &context.get_state().config;
	let kube_client = service::get_kubernetes_config(config).await?;

	super::create_ci_pipeline(
		ci_file,
		&repo_clone_url,
		repo_name,
		branch_name,
		kube_client,
	)
	.await?;

	Ok(())
}

// TODO: if possible make webhook_payload as a separate crate and automate
// struct update whenever there is a change in schema def of octokit/webhooks

/// These models are generated from
/// payload schema of [`octokit/webhooks`](https://github.com/octokit/webhooks/tree/master/payload-schemas)
/// using [`oxidecomputer/typify`](https://github.com/oxidecomputer/typify)
/// and then manually truncated for `PushEvent` alone.
pub mod webhook_payload {
	use serde::{Deserialize, Serialize};

	#[derive(Serialize, Deserialize, Debug, Clone)]
	pub struct PushEvent {
		/// The SHA of the most recent commit on `ref` after the push
		pub after: String,
		pub base_ref: Option<String>,
		/// The SHA of the most recent commit on `ref` before the push
		pub before: String,
		/// An array of commit objects describing the pushed commits.
		/// (Pushed commits are all commits that are included in the `compare`
		/// between the `before` commit and the `after` commit.)
		/// The array includes a maximum of 20 commits. If necessary,
		/// you can use the [Commits API](https://docs.github.com/en/rest/reference/repos#commits)
		/// to fetch additional commits. This limit is applied to timeline
		/// events only and isn't applied to webhook deliveries
		pub commits: Vec<Commit>,
		/// URL that shows the changes in this `ref` update, from the `before`
		/// commit to the `after` commit. For a newly created `ref` that is
		/// directly based on the default branch, this is the comparison
		/// between the head of the default branch and the `after` commit.
		/// Otherwise, this shows all commits until the `after` commit
		pub compare: String,
		/// Whether this push created the `ref`
		pub created: bool,
		/// Whether this push deleted the `ref`
		pub deleted: bool,
		/// Whether this push was a force push of the `ref`
		pub forced: bool,
		/// For pushes where `after` is or points to a commit object, an
		/// expanded representation of that commit. For pushes where `after`
		/// refers to an annotated tag object, an expanded representation of
		/// the commit pointed to by the annotated tag
		pub head_commit: Option<Commit>,
		#[serde(default)]
		pub installation: Option<InstallationLite>,
		#[serde(default)]
		pub organization: Option<Organization>,
		pub pusher: Committer,
		/// The full git ref that was pushed. Example: `refs/heads/main` or
		/// `refs/tags/v3.14.1`
		#[serde(rename = "ref")]
		pub ref_: String,
		pub repository: Repository,
		pub sender: User,
	}

	#[derive(Serialize, Deserialize, Debug, Clone)]
	pub struct User {
		pub avatar_url: String,
		#[serde(default)]
		pub email: Option<String>,
		pub events_url: String,
		pub followers_url: String,
		pub following_url: String,
		pub gists_url: String,
		pub gravatar_id: String,
		pub html_url: String,
		pub id: i64,
		pub login: String,
		#[serde(default)]
		pub name: Option<String>,
		pub node_id: String,
		pub organizations_url: String,
		pub received_events_url: String,
		pub repos_url: String,
		pub site_admin: bool,
		pub starred_url: String,
		pub subscriptions_url: String,
		#[serde(rename = "type")]
		pub type_: UserType,
		pub url: String,
	}
	#[derive(
		Serialize,
		Deserialize,
		Debug,
		Clone,
		Copy,
		PartialOrd,
		Ord,
		PartialEq,
		Eq,
		Hash,
	)]
	pub enum UserType {
		Bot,
		User,
		Organization,
	}

	#[derive(Serialize, Deserialize, Debug, Clone)]
	pub struct Commit {
		/// An array of files added in the commit
		pub added: Vec<String>,
		pub author: Committer,
		pub committer: Committer,
		/// Whether this commit is distinct from any that have been pushed
		/// before
		pub distinct: bool,
		pub id: String,
		/// The commit message
		pub message: String,
		/// An array of files modified by the commit
		pub modified: Vec<String>,
		/// An array of files removed in the commit
		pub removed: Vec<String>,
		/// The ISO 8601 timestamp of the commit
		pub timestamp: chrono::DateTime<chrono::offset::Utc>,
		pub tree_id: String,
		/// URL that points to the commit API resource
		pub url: String,
	}

	/// Metaproperties for Git author/committer information
	#[derive(Serialize, Deserialize, Debug, Clone)]
	pub struct Committer {
		#[serde(default)]
		pub date: Option<chrono::DateTime<chrono::offset::Utc>>,
		/// The git author's email address
		pub email: Option<String>,
		/// The git author's name
		pub name: String,
		#[serde(default)]
		pub username: Option<String>,
	}

	/// Installation
	#[derive(Serialize, Deserialize, Debug, Clone)]
	pub struct InstallationLite {
		/// The ID of the installation
		pub id: i64,
		pub node_id: String,
	}

	#[derive(Serialize, Deserialize, Debug, Clone)]
	pub struct Organization {
		pub avatar_url: String,
		pub description: Option<String>,
		pub events_url: String,
		pub hooks_url: String,
		#[serde(default)]
		pub html_url: Option<String>,
		pub id: i64,
		pub issues_url: String,
		pub login: String,
		pub members_url: String,
		pub node_id: String,
		pub public_members_url: String,
		pub repos_url: String,
		pub url: String,
	}

	/// A git repositor
	#[derive(Serialize, Deserialize, Debug, Clone)]
	pub struct Repository {
		/// Whether to allow auto-merge for pull requests
		#[serde(default)]
		pub allow_auto_merge: bool,
		/// Whether to allow private fork
		#[serde(default)]
		pub allow_forking: Option<bool>,
		/// Whether to allow merge commits for pull requests
		#[serde(default)]
		pub allow_merge_commit: bool,
		/// Whether to allow rebase merges for pull requests
		#[serde(default)]
		pub allow_rebase_merge: bool,
		/// Whether to allow squash merges for pull requests
		#[serde(default)]
		pub allow_squash_merge: bool,
		#[serde(default)]
		pub allow_update_branch: Option<bool>,
		pub archive_url: String,
		/// Whether the repository is archived
		pub archived: bool,
		pub assignees_url: String,
		pub blobs_url: String,
		pub branches_url: String,
		pub clone_url: String,
		pub collaborators_url: String,
		pub comments_url: String,
		pub commits_url: String,
		pub compare_url: String,
		pub contents_url: String,
		pub contributors_url: String,
		pub created_at: RepositoryCreatedAt,
		/// The default branch of the repository
		pub default_branch: String,
		/// Whether to delete head branches when pull requests are merge
		#[serde(default)]
		pub delete_branch_on_merge: bool,
		pub deployments_url: String,
		pub description: Option<String>,
		/// Returns whether or not this repository is disabled
		#[serde(default)]
		pub disabled: Option<bool>,
		pub downloads_url: String,
		pub events_url: String,
		pub fork: bool,
		pub forks: i64,
		pub forks_count: i64,
		pub forks_url: String,
		pub full_name: String,
		pub git_commits_url: String,
		pub git_refs_url: String,
		pub git_tags_url: String,
		pub git_url: String,
		/// Whether downloads are enabled
		pub has_downloads: bool,
		/// Whether issues are enabled
		pub has_issues: bool,
		pub has_pages: bool,
		/// Whether projects are enabled
		pub has_projects: bool,
		/// Whether the wiki is enabled
		pub has_wiki: bool,
		pub homepage: Option<String>,
		pub hooks_url: String,
		pub html_url: String,
		/// Unique identifier of the repositor
		pub id: i64,
		pub is_template: bool,
		pub issue_comment_url: String,
		pub issue_events_url: String,
		pub issues_url: String,
		pub keys_url: String,
		pub labels_url: String,
		pub language: Option<String>,
		pub languages_url: String,
		pub license: Option<License>,
		#[serde(default)]
		pub master_branch: Option<String>,
		pub merges_url: String,
		pub milestones_url: String,
		pub mirror_url: Option<String>,
		/// The name of the repository
		pub name: String,
		pub node_id: String,
		pub notifications_url: String,
		pub open_issues: i64,
		pub open_issues_count: i64,
		#[serde(default)]
		pub organization: Option<String>,
		pub owner: User,
		#[serde(default)]
		pub permissions: Option<RepositoryPermissions>,
		/// Whether the repository is private or public
		pub private: bool,
		#[serde(default)]
		pub public: Option<bool>,
		pub pulls_url: String,
		pub pushed_at: RepositoryPushedAt,
		pub releases_url: String,
		pub size: i64,
		pub ssh_url: String,
		#[serde(default)]
		pub stargazers: Option<i64>,
		pub stargazers_count: i64,
		pub stargazers_url: String,
		pub statuses_url: String,
		pub subscribers_url: String,
		pub subscription_url: String,
		pub svn_url: String,
		pub tags_url: String,
		pub teams_url: String,
		pub topics: Vec<String>,
		pub trees_url: String,
		pub updated_at: chrono::DateTime<chrono::offset::Utc>,
		pub url: String,
		#[serde(default)]
		pub use_squash_pr_title_as_default: Option<bool>,
		pub visibility: RepositoryVisibility,
		pub watchers: i64,
		pub watchers_count: i64,
	}

	#[derive(Serialize, Deserialize, Debug, Clone)]
	#[serde(untagged)]
	pub enum RepositoryCreatedAt {
		Variant0(i64),
		Variant1(chrono::DateTime<chrono::offset::Utc>),
	}

	#[derive(Serialize, Deserialize, Debug, Clone)]
	pub struct License {
		pub key: String,
		pub name: String,
		pub node_id: String,
		pub spdx_id: String,
		pub url: Option<String>,
	}

	#[derive(Serialize, Deserialize, Debug, Clone)]
	pub struct RepositoryPermissions {
		pub admin: bool,
		#[serde(default)]
		pub maintain: Option<bool>,
		pub pull: bool,
		pub push: bool,
		#[serde(default)]
		pub triage: Option<bool>,
	}

	#[derive(Serialize, Deserialize, Debug, Clone)]
	#[serde(untagged)]
	pub enum RepositoryPushedAt {
		Variant0(i64),
		Variant1(chrono::DateTime<chrono::offset::Utc>),
		Variant2,
	}

	#[derive(
		Serialize,
		Deserialize,
		Debug,
		Clone,
		Copy,
		PartialOrd,
		Ord,
		PartialEq,
		Eq,
		Hash,
	)]
	pub enum RepositoryVisibility {
		#[serde(rename = "public")]
		Public,
		#[serde(rename = "private")]
		Private,
		#[serde(rename = "internal")]
		Internal,
	}
}
