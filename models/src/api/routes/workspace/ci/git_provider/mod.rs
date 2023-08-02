mod activate_repo;
mod cancel_build;
mod deactivate_repo;
mod get_build_info;
mod get_build_logs;
mod get_patr_ci_file;
mod get_repo_info;
mod github_auth;
mod github_auth_callback;
mod github_sign_out;
mod list_git_providers;
mod list_git_ref_for_repo;
mod list_repo_builds;
mod list_repos;
mod restart_build;
mod start_build_for_branch;
mod sync_repos;
mod write_patr_ci_file;

use chrono::Utc;
use serde::{Deserialize, Serialize};

pub use self::{
	activate_repo::*,
	cancel_build::*,
	deactivate_repo::*,
	get_build_info::*,
	get_build_logs::*,
	get_patr_ci_file::*,
	get_repo_info::*,
	github_auth::*,
	github_auth_callback::*,
	github_sign_out::*,
	list_git_providers::*,
	list_git_ref_for_repo::*,
	list_repo_builds::*,
	list_repos::*,
	restart_build::*,
	start_build_for_branch::*,
	sync_repos::*,
	write_patr_ci_file::*,
};
use crate::utils::{DateTime, Uuid};

#[cfg(feature = "server")]
#[derive(sqlx::Type, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[sqlx(type_name = "CI_GIT_PROVIDER_TYPE", rename_all = "snake_case")]
#[serde(rename_all = "camelCase")]
pub enum GitProviderType {
	Github,
}

#[cfg(not(feature = "server"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GitProviderType {
	Github,
}

#[cfg(feature = "server")]
#[derive(
	sqlx::Type, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash,
)]
#[sqlx(type_name = "CI_REPO_STATUS", rename_all = "snake_case")]
#[serde(rename_all = "camelCase")]
pub enum RepoStatus {
	Active,
	Inactive,
	Deleted,
}

#[cfg(not(feature = "server"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "camelCase")]
pub enum RepoStatus {
	Active,
	Inactive,
	Deleted,
}

#[cfg(feature = "server")]
#[derive(sqlx::Type, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[sqlx(type_name = "CI_BUILD_STATUS", rename_all = "snake_case")]
#[serde(rename_all = "camelCase")]
pub enum BuildStatus {
	WaitingToStart,
	Running,
	Succeeded,
	Cancelled,
	Errored,
}

#[cfg(not(feature = "server"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BuildStatus {
	WaitingToStart,
	Running,
	Succeeded,
	Cancelled,
	Errored,
}

#[cfg(feature = "server")]
#[derive(sqlx::Type, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[sqlx(type_name = "CI_BUILD_STEP_STATUS", rename_all = "snake_case")]
#[serde(rename_all = "camelCase")]
pub enum BuildStepStatus {
	WaitingToStart,
	Running,
	Succeeded,
	Cancelled,
	Errored,
	SkippedDepError,
}

#[cfg(not(feature = "server"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BuildStepStatus {
	WaitingToStart,
	Running,
	Succeeded,
	Cancelled,
	Errored,
	SkippedDepError,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildDetails {
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
	pub runner_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Step {
	pub step_id: u32,
	pub step_name: String,
	pub base_image: String,
	pub commands: String,
	pub status: BuildStepStatus,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub started: Option<DateTime<Utc>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub finished: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildLogs {
	pub log: String,
	pub time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryDetails {
	pub id: String,
	pub name: String,
	pub repo_owner: String,
	pub clone_url: String,
	pub status: RepoStatus,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub runner_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitProvider {
	pub id: Uuid,
	pub domain_name: String,
	pub git_provider_type: GitProviderType,
	pub login_name: Option<String>,
	pub is_syncing: bool,
	pub last_synced: Option<DateTime<Utc>>,
	pub is_deleted: bool,
}
