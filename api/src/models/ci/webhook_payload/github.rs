// These types are auto- generated from the payload schema of [`octokit/webhooks`](https://github.com/octokit/webhooks/tree/master/payload-schemas)
// using [`oxidecomputer/typify`](https://github.com/oxidecomputer/typify) then manually modified for Patr needs.

// currently types are based on json schema file => https://unpkg.com/@octokit/webhooks-schemas@6.3.6/schema.json

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Event {
	Push(PushEvent),
	PullRequestOpened(PullRequestOpened),
	PullRequestSynchronize(PullRequestSynchronize),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PushEvent {
	#[doc = "The SHA of the most recent commit on `ref` after the push."]
	pub after: String,
	pub base_ref: Option<String>,
	#[doc = "The SHA of the most recent commit on `ref` before the push."]
	pub before: String,
	#[doc = "An array of commit objects describing the pushed commits. (Pushed commits are all commits that are included in the `compare` between the `before` commit and the `after` commit.) The array includes a maximum of 20 commits. If necessary, you can use the [Commits API](https://docs.github.com/en/rest/reference/repos#commits) to fetch additional commits. This limit is applied to timeline events only and isn't applied to webhook deliveries."]
	pub commits: Vec<Commit>,
	#[doc = "URL that shows the changes in this `ref` update, from the `before` commit to the `after` commit. For a newly created `ref` that is directly based on the default branch, this is the comparison between the head of the default branch and the `after` commit. Otherwise, this shows all commits until the `after` commit."]
	pub compare: String,
	#[doc = "Whether this push created the `ref`."]
	pub created: bool,
	#[doc = "Whether this push deleted the `ref`."]
	pub deleted: bool,
	#[doc = "Whether this push was a force push of the `ref`."]
	pub forced: bool,
	#[doc = "For pushes where `after` is or points to a commit object, an expanded representation of that commit. For pushes where `after` refers to an annotated tag object, an expanded representation of the commit pointed to by the annotated tag."]
	pub head_commit: Option<Commit>,
	#[serde(default)]
	pub installation: Option<InstallationLite>,
	#[serde(default)]
	pub organization: Option<Organization>,
	pub pusher: Committer,
	#[doc = "The full git ref that was pushed. Example: `refs/heads/main` or `refs/tags/v3.14.1`."]
	#[serde(rename = "ref")]
	pub ref_: String,
	pub repository: Repository,
	pub sender: User,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Commit {
	#[doc = "An array of files added in the commit. For extremely large commits where GitHub is unable to calculate this list in a timely manner, this may be empty even if files were added."]
	pub added: Vec<String>,
	pub author: Committer,
	pub committer: Committer,
	#[doc = "Whether this commit is distinct from any that have been pushed before."]
	pub distinct: bool,
	pub id: String,
	#[doc = "The commit message."]
	pub message: String,
	#[doc = "An array of files modified by the commit. For extremely large commits where GitHub is unable to calculate this list in a timely manner, this may be empty even if files were modified."]
	pub modified: Vec<String>,
	#[doc = "An array of files removed in the commit. For extremely large commits where GitHub is unable to calculate this list in a timely manner, this may be empty even if files were removed."]
	pub removed: Vec<String>,
	#[doc = "The ISO 8601 timestamp of the commit."]
	pub timestamp: chrono::DateTime<chrono::offset::Utc>,
	pub tree_id: String,
	#[doc = "URL that points to the commit API resource."]
	pub url: String,
}

#[doc = "Installation"]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InstallationLite {
	#[doc = "The ID of the installation."]
	pub id: i64,
	pub node_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
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

#[doc = "Metaproperties for Git author/committer information."]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Committer {
	#[serde(default)]
	pub date: Option<chrono::DateTime<chrono::offset::Utc>>,
	#[doc = "The git author's email address."]
	pub email: Option<String>,
	#[doc = "The git author's name."]
	pub name: String,
	#[serde(default)]
	pub username: Option<String>,
}
#[doc = "A git repository"]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Repository {
	#[doc = "Whether to allow auto-merge for pull requests."]
	#[serde(default)]
	pub allow_auto_merge: bool,
	#[doc = "Whether to allow private forks"]
	#[serde(default)]
	pub allow_forking: Option<bool>,
	#[doc = "Whether to allow merge commits for pull requests."]
	#[serde(default)]
	pub allow_merge_commit: bool,
	#[doc = "Whether to allow rebase merges for pull requests."]
	#[serde(default)]
	pub allow_rebase_merge: bool,
	#[doc = "Whether to allow squash merges for pull requests."]
	#[serde(default)]
	pub allow_squash_merge: bool,
	#[serde(default)]
	pub allow_update_branch: Option<bool>,
	#[doc = "A template for the API URL to download the repository as an archive."]
	pub archive_url: String,
	#[doc = "Whether the repository is archived."]
	pub archived: bool,
	#[doc = "A template for the API URL to list the available assignees for issues in the repository."]
	pub assignees_url: String,
	#[doc = "A template for the API URL to create or retrieve a raw Git blob in the repository."]
	pub blobs_url: String,
	#[doc = "A template for the API URL to get information about branches in the repository."]
	pub branches_url: String,
	pub clone_url: String,
	#[doc = "A template for the API URL to get information about collaborators of the repository."]
	pub collaborators_url: String,
	#[doc = "A template for the API URL to get information about comments on the repository."]
	pub comments_url: String,
	#[doc = "A template for the API URL to get information about commits on the repository."]
	pub commits_url: String,
	#[doc = "A template for the API URL to compare two commits or refs."]
	pub compare_url: String,
	#[doc = "A template for the API URL to get the contents of the repository."]
	pub contents_url: String,
	#[doc = "A template for the API URL to list the contributors to the repository."]
	pub contributors_url: String,
	pub created_at: RepositoryCreatedAt,
	#[doc = "The default branch of the repository."]
	pub default_branch: String,
	#[doc = "Whether to delete head branches when pull requests are merged"]
	#[serde(default)]
	pub delete_branch_on_merge: bool,
	#[doc = "The API URL to list the deployments of the repository."]
	pub deployments_url: String,
	#[doc = "The repository description."]
	pub description: Option<String>,
	#[doc = "Returns whether or not this repository is disabled."]
	#[serde(default)]
	pub disabled: Option<bool>,
	#[doc = "The API URL to list the downloads on the repository."]
	pub downloads_url: String,
	#[doc = "The API URL to list the events of the repository."]
	pub events_url: String,
	#[doc = "Whether the repository is a fork."]
	pub fork: bool,
	pub forks: i64,
	pub forks_count: i64,
	#[doc = "The API URL to list the forks of the repository."]
	pub forks_url: String,
	#[doc = "The full, globally unique, name of the repository."]
	pub full_name: String,
	#[doc = "A template for the API URL to get information about Git commits of the repository."]
	pub git_commits_url: String,
	#[doc = "A template for the API URL to get information about Git refs of the repository."]
	pub git_refs_url: String,
	#[doc = "A template for the API URL to get information about Git tags of the repository."]
	pub git_tags_url: String,
	pub git_url: String,
	#[doc = "Whether downloads are enabled."]
	pub has_downloads: bool,
	#[doc = "Whether issues are enabled."]
	pub has_issues: bool,
	pub has_pages: bool,
	#[doc = "Whether projects are enabled."]
	pub has_projects: bool,
	#[doc = "Whether the wiki is enabled."]
	pub has_wiki: bool,
	pub homepage: Option<String>,
	#[doc = "The API URL to list the hooks on the repository."]
	pub hooks_url: String,
	#[doc = "The URL to view the repository on GitHub.com."]
	pub html_url: String,
	#[doc = "Unique identifier of the repository"]
	pub id: i64,
	pub is_template: bool,
	#[doc = "A template for the API URL to get information about issue comments on the repository."]
	pub issue_comment_url: String,
	#[doc = "A template for the API URL to get information about issue events on the repository."]
	pub issue_events_url: String,
	#[doc = "A template for the API URL to get information about issues on the repository."]
	pub issues_url: String,
	#[doc = "A template for the API URL to get information about deploy keys on the repository."]
	pub keys_url: String,
	#[doc = "A template for the API URL to get information about labels of the repository."]
	pub labels_url: String,
	pub language: Option<String>,
	#[doc = "The API URL to get information about the languages of the repository."]
	pub languages_url: String,
	pub license: Option<License>,
	#[serde(default)]
	pub master_branch: Option<String>,
	#[doc = "The API URL to merge branches in the repository."]
	pub merges_url: String,
	#[doc = "A template for the API URL to get information about milestones of the repository."]
	pub milestones_url: String,
	pub mirror_url: Option<String>,
	#[doc = "The name of the repository."]
	pub name: String,
	#[doc = "The GraphQL identifier of the repository."]
	pub node_id: String,
	#[doc = "A template for the API URL to get information about notifications on the repository."]
	pub notifications_url: String,
	pub open_issues: i64,
	pub open_issues_count: i64,
	#[serde(default)]
	pub organization: Option<String>,
	pub owner: User,
	#[serde(default)]
	pub permissions: Option<RepositoryPermissions>,
	#[doc = "Whether the repository is private or public."]
	pub private: bool,
	#[serde(default)]
	pub public: Option<bool>,
	#[doc = "A template for the API URL to get information about pull requests on the repository."]
	pub pulls_url: String,
	pub pushed_at: RepositoryPushedAt,
	#[doc = "A template for the API URL to get information about releases on the repository."]
	pub releases_url: String,
	pub size: i64,
	pub ssh_url: String,
	#[serde(default)]
	pub stargazers: Option<i64>,
	pub stargazers_count: i64,
	#[doc = "The API URL to list the stargazers on the repository."]
	pub stargazers_url: String,
	#[doc = "A template for the API URL to get information about statuses of a commit."]
	pub statuses_url: String,
	#[doc = "The API URL to list the subscribers on the repository."]
	pub subscribers_url: String,
	#[doc = "The API URL to subscribe to notifications for this repository."]
	pub subscription_url: String,
	pub svn_url: String,
	#[doc = "The API URL to get information about tags on the repository."]
	pub tags_url: String,
	#[doc = "The API URL to list the teams on the repository."]
	pub teams_url: String,
	pub topics: Vec<String>,
	#[doc = "A template for the API URL to create or retrieve a raw Git tree of the repository."]
	pub trees_url: String,
	pub updated_at: chrono::DateTime<chrono::offset::Utc>,
	#[doc = "The URL to get more information about the repository from the GitHub API."]
	pub url: String,
	#[serde(default)]
	pub use_squash_pr_title_as_default: Option<bool>,
	pub visibility: RepositoryVisibility,
	pub watchers: i64,
	pub watchers_count: i64,
	pub web_commit_signoff_required: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
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
	Clone,
	Copy,
	Debug,
	Deserialize,
	Eq,
	Hash,
	Ord,
	PartialEq,
	PartialOrd,
	Serialize,
)]
pub enum UserType {
	Bot,
	User,
	Organization,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RepositoryCreatedAt {
	Variant0(i64),
	Variant1(chrono::DateTime<chrono::offset::Utc>),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct License {
	pub key: String,
	pub name: String,
	pub node_id: String,
	pub spdx_id: String,
	pub url: Option<String>,
}
#[derive(
	Clone,
	Copy,
	Debug,
	Deserialize,
	Eq,
	Hash,
	Ord,
	PartialEq,
	PartialOrd,
	Serialize,
)]
pub enum RepositoryVisibility {
	#[serde(rename = "public")]
	Public,
	#[serde(rename = "private")]
	Private,
	#[serde(rename = "internal")]
	Internal,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RepositoryPushedAt {
	Variant0(i64),
	Variant1(chrono::DateTime<chrono::offset::Utc>),
	Variant2,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RepositoryPermissions {
	pub admin: bool,
	#[serde(default)]
	pub maintain: Option<bool>,
	pub pull: bool,
	pub push: bool,
	#[serde(default)]
	pub triage: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PullRequestOpened {
	pub action: PullRequestOpenedAction,
	#[serde(default)]
	pub installation: Option<InstallationLite>,
	#[doc = "The pull request number."]
	pub number: i64,
	#[serde(default)]
	pub organization: Option<Organization>,
	pub pull_request: PullRequest,
	pub repository: Repository,
	pub sender: User,
}

#[derive(
	Clone,
	Copy,
	Debug,
	Deserialize,
	Eq,
	Hash,
	Ord,
	PartialEq,
	PartialOrd,
	Serialize,
)]
pub enum PullRequestOpenedAction {
	#[serde(rename = "opened")]
	Opened,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PullRequestSynchronize {
	pub action: PullRequestSynchronizeAction,
	pub after: String,
	pub before: String,
	#[serde(default)]
	pub installation: Option<InstallationLite>,
	#[doc = "The pull request number."]
	pub number: i64,
	#[serde(default)]
	pub organization: Option<Organization>,
	pub pull_request: PullRequest,
	pub repository: Repository,
	pub sender: User,
}

#[derive(
	Clone,
	Copy,
	Debug,
	Deserialize,
	Eq,
	Hash,
	Ord,
	PartialEq,
	PartialOrd,
	Serialize,
)]
pub enum PullRequestSynchronizeAction {
	#[serde(rename = "synchronize")]
	Synchronize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PullRequest {
	pub active_lock_reason: Option<PullRequestActiveLockReason>,
	pub additions: i64,
	pub assignee: Option<User>,
	pub assignees: Vec<User>,
	pub author_association: AuthorAssociation,
	pub auto_merge: Option<AutoMerge>,
	pub base: PullRequestBase,
	pub body: Option<String>,
	pub changed_files: i64,
	pub closed_at: Option<chrono::DateTime<chrono::offset::Utc>>,
	pub comments: i64,
	pub comments_url: String,
	pub commits: i64,
	pub commits_url: String,
	pub created_at: chrono::DateTime<chrono::offset::Utc>,
	pub deletions: i64,
	pub diff_url: String,
	#[doc = "Indicates whether or not the pull request is a draft."]
	pub draft: bool,
	pub head: PullRequestHead,
	pub html_url: String,
	pub id: i64,
	pub issue_url: String,
	pub labels: Vec<Label>,
	#[serde(rename = "_links")]
	pub links: PullRequestLinks,
	pub locked: bool,
	#[doc = "Indicates whether maintainers can modify the pull request."]
	pub maintainer_can_modify: bool,
	pub merge_commit_sha: Option<String>,
	pub mergeable: Option<bool>,
	pub mergeable_state: String,
	pub merged: Option<bool>,
	pub merged_at: Option<chrono::DateTime<chrono::offset::Utc>>,
	pub merged_by: Option<User>,
	pub milestone: Option<Milestone>,
	pub node_id: String,
	#[doc = "Number uniquely identifying the pull request within its repository."]
	pub number: i64,
	pub patch_url: String,
	pub rebaseable: Option<bool>,
	pub requested_reviewers: Vec<PullRequestRequestedReviewersItem>,
	pub requested_teams: Vec<Team>,
	pub review_comment_url: String,
	pub review_comments: i64,
	pub review_comments_url: String,
	#[doc = "State of this Pull Request. Either `open` or `closed`."]
	pub state: PullRequestState,
	pub statuses_url: String,
	#[doc = "The title of the pull request."]
	pub title: String,
	pub updated_at: chrono::DateTime<chrono::offset::Utc>,
	pub url: String,
	pub user: User,
}

#[derive(
	Clone,
	Copy,
	Debug,
	Deserialize,
	Eq,
	Hash,
	Ord,
	PartialEq,
	PartialOrd,
	Serialize,
)]
pub enum PullRequestActiveLockReason {
	#[serde(rename = "resolved")]
	Resolved,
	#[serde(rename = "off-topic")]
	OffTopic,
	#[serde(rename = "too heated")]
	TooHeated,
	#[serde(rename = "spam")]
	Spam,
}

#[doc = "How the author is associated with the repository."]
#[derive(
	Clone,
	Copy,
	Debug,
	Deserialize,
	Eq,
	Hash,
	Ord,
	PartialEq,
	PartialOrd,
	Serialize,
)]
pub enum AuthorAssociation {
	#[serde(rename = "COLLABORATOR")]
	Collaborator,
	#[serde(rename = "CONTRIBUTOR")]
	Contributor,
	#[serde(rename = "FIRST_TIMER")]
	FirstTimer,
	#[serde(rename = "FIRST_TIME_CONTRIBUTOR")]
	FirstTimeContributor,
	#[serde(rename = "MANNEQUIN")]
	Mannequin,
	#[serde(rename = "MEMBER")]
	Member,
	#[serde(rename = "NONE")]
	None,
	#[serde(rename = "OWNER")]
	Owner,
}

#[doc = "The status of auto merging a pull request."]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AutoMerge {
	#[doc = "Commit message for the merge commit."]
	pub commit_message: String,
	#[doc = "Title for the merge commit message."]
	pub commit_title: String,
	pub enabled_by: User,
	#[doc = "The merge method to use."]
	pub merge_method: AutoMergeMergeMethod,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PullRequestBase {
	pub label: String,
	#[serde(rename = "ref")]
	pub ref_: String,
	pub repo: Repository,
	pub sha: String,
	pub user: User,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PullRequestHead {
	pub label: String,
	#[serde(rename = "ref")]
	pub ref_: String,
	pub repo: Repository,
	pub sha: String,
	pub user: User,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Label {
	#[doc = "6-character hex code, without the leading #, identifying the color"]
	pub color: String,
	pub default: bool,
	pub description: Option<String>,
	pub id: i64,
	#[doc = "The name of the label."]
	pub name: String,
	pub node_id: String,
	#[doc = "URL for the label"]
	pub url: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PullRequestLinks {
	pub comments: Link,
	pub commits: Link,
	pub html: Link,
	pub issue: Link,
	pub review_comment: Link,
	pub review_comments: Link,
	#[serde(rename = "self")]
	pub self_: Link,
	pub statuses: Link,
}

#[doc = "A collection of related issues and pull requests."]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Milestone {
	pub closed_at: Option<chrono::DateTime<chrono::offset::Utc>>,
	pub closed_issues: i64,
	pub created_at: chrono::DateTime<chrono::offset::Utc>,
	pub creator: User,
	pub description: Option<String>,
	pub due_on: Option<chrono::DateTime<chrono::offset::Utc>>,
	pub html_url: String,
	pub id: i64,
	pub labels_url: String,
	pub node_id: String,
	#[doc = "The number of the milestone."]
	pub number: i64,
	pub open_issues: i64,
	#[doc = "The state of the milestone."]
	pub state: MilestoneState,
	#[doc = "The title of the milestone."]
	pub title: String,
	pub updated_at: chrono::DateTime<chrono::offset::Utc>,
	pub url: String,
}

#[doc = "The state of the milestone."]
#[derive(
	Clone,
	Copy,
	Debug,
	Deserialize,
	Eq,
	Hash,
	Ord,
	PartialEq,
	PartialOrd,
	Serialize,
)]
pub enum MilestoneState {
	#[serde(rename = "open")]
	Open,
	#[serde(rename = "closed")]
	Closed,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PullRequestRequestedReviewersItem {
	User(User),
	Team(Team),
}

#[doc = "Groups of organization members that gives permissions on specified repositories."]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Team {
	#[doc = "Description of the team"]
	pub description: Option<String>,
	pub html_url: String,
	#[doc = "Unique identifier of the team"]
	pub id: i64,
	pub members_url: String,
	#[doc = "Name of the team"]
	pub name: String,
	pub node_id: String,
	#[serde(default)]
	pub parent: Option<TeamParent>,
	#[doc = "Permission that the team will have for its repositories"]
	pub permission: String,
	pub privacy: TeamPrivacy,
	pub repositories_url: String,
	pub slug: String,
	#[doc = "URL for the team"]
	pub url: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TeamParent {
	#[doc = "Description of the team"]
	pub description: Option<String>,
	pub html_url: String,
	#[doc = "Unique identifier of the team"]
	pub id: i64,
	pub members_url: String,
	#[doc = "Name of the team"]
	pub name: String,
	pub node_id: String,
	#[doc = "Permission that the team will have for its repositories"]
	pub permission: String,
	pub privacy: TeamParentPrivacy,
	pub repositories_url: String,
	pub slug: String,
	#[doc = "URL for the team"]
	pub url: String,
}

#[derive(
	Clone,
	Copy,
	Debug,
	Deserialize,
	Eq,
	Hash,
	Ord,
	PartialEq,
	PartialOrd,
	Serialize,
)]
pub enum TeamParentPrivacy {
	#[serde(rename = "open")]
	Open,
	#[serde(rename = "closed")]
	Closed,
	#[serde(rename = "secret")]
	Secret,
}

#[derive(
	Clone,
	Copy,
	Debug,
	Deserialize,
	Eq,
	Hash,
	Ord,
	PartialEq,
	PartialOrd,
	Serialize,
)]
pub enum TeamPrivacy {
	#[serde(rename = "open")]
	Open,
	#[serde(rename = "closed")]
	Closed,
	#[serde(rename = "secret")]
	Secret,
}

#[doc = "State of this Pull Request. Either `open` or `closed`."]
#[derive(
	Clone,
	Copy,
	Debug,
	Deserialize,
	Eq,
	Hash,
	Ord,
	PartialEq,
	PartialOrd,
	Serialize,
)]
pub enum PullRequestState {
	#[serde(rename = "open")]
	Open,
	#[serde(rename = "closed")]
	Closed,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Link {
	pub href: String,
}

#[doc = "The merge method to use."]
#[derive(
	Clone,
	Copy,
	Debug,
	Deserialize,
	Eq,
	Hash,
	Ord,
	PartialEq,
	PartialOrd,
	Serialize,
)]
pub enum AutoMergeMergeMethod {
	#[serde(rename = "merge")]
	Merge,
	#[serde(rename = "squash")]
	Squash,
	#[serde(rename = "rebase")]
	Rebase,
}
