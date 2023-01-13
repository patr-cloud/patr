use std::collections::HashMap;

use api_models::{
	models::workspace::ci::git_provider::{BuildStatus, Ref, RefType},
	utils::Uuid,
};
use chrono::Utc;
use eve_rs::AsError;
use hmac::{Hmac, Mac};
use octorust::{
	auth::Credentials,
	types::{
		GitCreateCommitRequest,
		GitCreateCommitRequestCommitter,
		GitCreateRefRequest,
		GitCreateTagRequestType,
		GitCreateTreeRequest,
		GitCreateTreeRequestData,
		GitCreateTreeRequestMode,
		GitUpdateRefRequest,
		Order,
		ReposCreateCommitStatusRequest,
		ReposCreateCommitStatusRequestState,
		ReposListOrgSort,
		ReposListVisibility,
	},
};
use sha2::Sha256;

use super::MutableRepoValues;
use crate::{db, models::ci::EventType, service, utils::Error, Database};

type HmacSha256 = Hmac<Sha256>;

/// Returns error if payload signature is different from header signature
pub fn verify_github_payload_signature_256(
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

pub async fn create_build_for_repo(
	connection: &mut <Database as sqlx::Database>::Connection,
	repo_id: &Uuid,
	event_type: &EventType,
) -> Result<i64, Error> {
	let build_num = match &event_type {
		EventType::Commit(commit) => {
			db::generate_new_build_for_repo(
				connection,
				repo_id,
				&format!("refs/heads/{}", commit.committed_branch_name),
				&commit.commit_sha,
				BuildStatus::Running,
				&Utc::now(),
				&commit.author,
				commit.commit_message.as_deref(),
				None,
			)
			.await?
		}
		EventType::Tag(tag) => {
			db::generate_new_build_for_repo(
				connection,
				repo_id,
				&format!("refs/tags/{}", tag.tag_name),
				&tag.commit_sha,
				BuildStatus::Running,
				&Utc::now(),
				&tag.author,
				tag.commit_message.as_deref(),
				None,
			)
			.await?
		}
		EventType::PullRequest(pull_request) => {
			db::generate_new_build_for_repo(
				connection,
				repo_id,
				&format!("refs/pull/{}", pull_request.pr_number),
				&pull_request.commit_sha,
				BuildStatus::Running,
				&Utc::now(),
				&pull_request.author,
				None,
				Some(&pull_request.pr_title),
			)
			.await?
		}
	};

	Ok(build_num)
}

pub async fn list_git_ref_for_repo(
	owner_name: &str,
	repo_name: &str,
	access_token: &str,
) -> Result<Vec<Ref>, Error> {
	let github_client = octorust::Client::new(
		"patr",
		Credentials::Token(access_token.to_owned()),
	)
	.map_err(|err| {
		log::info!("Error while initializing git client: {err:#}");
		Error::empty().status(500)
	})?;

	let branches = github_client
		.repos()
		.list_all_branches(owner_name, repo_name, false)
		.await
		.map_err(|err| {
			log::info!("Error while fetching git branches: {err:#}");
			Error::empty().status(500)
		})?
		.into_iter()
		.map(|branch| Ref {
			type_: RefType::Branch,
			name: branch.name,
			latest_commit_sha: branch.commit.sha,
		});

	let tags = github_client
		.repos()
		.list_all_tags(owner_name, repo_name)
		.await
		.map_err(|err| {
			log::info!("Error while fetching git tags: {err:#}");
			Error::empty().status(500)
		})?
		.into_iter()
		.map(|tag| Ref {
			type_: RefType::Tag,
			name: tag.name,
			latest_commit_sha: tag.commit.sha,
		});

	Ok(branches.chain(tags).collect())
}

pub async fn fetch_ci_file_content_from_github_repo(
	owner_name: &str,
	repo_name: &str,
	git_ref: &str, // name of the commit/branch/tag
	access_token: &str,
) -> Result<Vec<u8>, Error> {
	let github_client = octorust::Client::new(
		"patr",
		Credentials::Token(access_token.to_owned()),
	)
	.map_err(|err| {
		log::info!("error while octorust init: {err:#}");
		err
	})
	.ok()
	.status(500)
	.body("error while initailizing octorust")?;

	let ci_file = github_client
		.repos()
		.get_content_file(owner_name, repo_name, "patr.yml", git_ref)
		.await
		.ok()
		.status(400)
		.body("patr.yml file is not defined")?;

	let ci_file = reqwest::Client::new()
		.get(ci_file.download_url)
		.bearer_auth(access_token)
		.send()
		.await?
		.bytes()
		.await?
		.to_vec();

	Ok(ci_file)
}

pub async fn write_ci_file_content_to_github_repo(
	owner_name: &str,
	repo_name: &str,
	commit_message: String,
	parent_commit_sha: String,
	branch_name: String,
	ci_file_content: String,
	access_token: &str,
) -> Result<(), Error> {
	let github_client = octorust::Client::new(
		"patr",
		Credentials::Token(access_token.to_owned()),
	)
	.map_err(|err| {
		log::info!("Error while initializing git client: {err:#}");
		Error::empty().status(500)
	})?;

	// get base tree for parent commit
	let parent_tree_sha = github_client
		.repos()
		.get_commit(owner_name, repo_name, 1, 30, &parent_commit_sha)
		.await
		.map_err(|err| {
			log::info!(
				"Error while getting base tree for parent commit: {err:#}"
			);
			Error::empty().status(500)
		})?
		.commit
		.tree
		.sha;

	// create new tree from base tree
	let new_tree_sha = github_client
		.git()
		.create_tree(
			owner_name,
			repo_name,
			&GitCreateTreeRequestData {
				base_tree: parent_tree_sha,
				tree: vec![GitCreateTreeRequest {
					path: "patr.yml".into(),
					mode: Some(GitCreateTreeRequestMode::FileBlob),
					type_: Some(GitCreateTagRequestType::Blob),
					sha: "".into(),
					content: ci_file_content,
				}],
			},
		)
		.await
		.map_err(|err| {
			log::info!("Error while creating new tree from base tree: {err:#}");
			Error::empty().status(500)
		})?
		.sha;

	// create new commit
	let new_commit_sha = github_client
		.git()
		.create_commit(
			owner_name,
			repo_name,
			&GitCreateCommitRequest {
				committer: Some(GitCreateCommitRequestCommitter {
					date: None,
					email: "patr-ci@patr.cloud".into(),
					name: "patr-ci".into(),
				}),
				author: None,
				message: commit_message,
				parents: vec![parent_commit_sha],
				signature: "".into(),
				tree: new_tree_sha,
			},
		)
		.await
		.map_err(|err| {
			log::info!("Error while creating new commit for new tree: {err:#}");
			Error::empty().status(500)
		})?
		.sha;

	// point this commit to the given branch name
	let branch_name = format!("heads/{}", branch_name);
	let branch_exists = github_client
		.git()
		.get_ref(owner_name, repo_name, &branch_name)
		.await
		.is_ok();

	if branch_exists {
		// update branch reference
		github_client
			.git()
			.update_ref(
				owner_name,
				repo_name,
				&branch_name,
				&GitUpdateRefRequest {
					force: None, // only fast forward is supported
					sha: new_commit_sha,
				},
			)
			.await
			.map_err(|err| {
				log::info!(
					"Error while pointing existing branch head to new commit: {err:#}"
				);
				Error::empty().status(500)
			})?;
	} else {
		// create branch reference
		github_client
			.git()
			.create_ref(
				owner_name,
				repo_name,
				&GitCreateRefRequest {
					key: "".into(),
					ref_: format!("refs/{}", branch_name),
					sha: new_commit_sha,
				},
			)
			.await
			.map_err(|err| {
				log::info!(
					"Error while pointing branch head to new commit: {err:#}"
				);
				Error::empty().status(500)
			})?;
	}

	Ok(())
}

pub async fn sync_github_repos(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	git_provider_id: &Uuid,
	github_access_token: String,
	request_id: &Uuid,
) -> Result<(), eve_rs::Error<()>> {
	let repos_in_db =
		db::list_repos_for_git_provider(connection, git_provider_id)
			.await?
			.into_iter()
			.map(|repo| {
				(
					repo.git_provider_repo_uid,
					MutableRepoValues {
						repo_owner: repo.repo_owner,
						repo_name: repo.repo_name,
						repo_clone_url: repo.clone_url,
					},
				)
			})
			.collect::<HashMap<_, _>>();

	let github_client =
		octorust::Client::new("patr", Credentials::Token(github_access_token))
			.map_err(|err| {
				log::info!("error while octorust init: {err:#}");
				err
			})
			.ok()
			.status(500)?;

	let repos_in_github = github_client
		    .repos()
		    .list_all_for_authenticated_user(
			    Some(ReposListVisibility::All),
			    "",
			    None,
			    ReposListOrgSort::Created,
			    Order::Desc,
			    None,
			    None,
		    )
		    .await
		    .map_err(|err| {
			    log::info!("error while getting repo list: {err:#}");
			    err
		    })
		    .ok()
		    .status(500)?
		    .into_iter()
		    .filter_map(|repo| {
			    let splitted_name = repo.full_name.rsplit_once('/');
			    if let Some((repo_owner, repo_name)) =  splitted_name {
				    Some((
					    repo.id.to_string(),
					    MutableRepoValues {
						    repo_owner: repo_owner.to_string(),
						    repo_name: repo_name.to_string(),
						    repo_clone_url: repo.clone_url,
					    }
				    ))
			    } else {
				    log::trace!("request_id: {} - Error while getting repo owner and repo name from full name {}", request_id, repo.full_name);
				    None
			    }
		    })
		    .collect::<HashMap<_, _>>();

	service::sync_repos_in_db(
		connection,
		workspace_id,
		git_provider_id,
		repos_in_github,
		repos_in_db,
		request_id,
	)
	.await?;

	Ok(())
}

pub enum CommitStatus {
	// build is started and running
	Running,
	// build finished and success
	Success,
	// build finished and failure
	Failed,
	// build has been errored ie cancelled / internal error
	Errored,
}

impl CommitStatus {
	pub fn commit_state(&self) -> ReposCreateCommitStatusRequestState {
		match self {
			Self::Running => ReposCreateCommitStatusRequestState::Pending,
			Self::Success => ReposCreateCommitStatusRequestState::Success,
			Self::Failed => ReposCreateCommitStatusRequestState::Failure,
			Self::Errored => ReposCreateCommitStatusRequestState::Error,
		}
	}

	pub fn description(&self) -> &str {
		match self {
			Self::Running => "Build is running",
			Self::Success => "Build succeeded",
			Self::Failed => "Build failed",
			Self::Errored => "Error occurred",
		}
	}
}

pub async fn update_github_commit_status_for_build(
	connection: &mut <Database as sqlx::Database>::Connection,
	repo_id: &Uuid,
	build_num: i64,
	status: CommitStatus,
) -> Result<(), Error> {
	let repo = db::get_repo_using_patr_repo_id(connection, repo_id)
		.await?
		.status(500)?;

	let (_login_name, access_token) =
		db::get_git_provider_details_by_id(connection, &repo.git_provider_id)
			.await?
			.and_then(|git_provider| {
				git_provider.login_name.zip(git_provider.password)
			})
			.status(500)?;

	let commit_sha =
		db::get_build_details_for_build(connection, repo_id, build_num)
			.await?
			.status(500)?
			.git_commit;

	log::debug!(
		"Updating commit status in github for {}/{}@{}",
		repo.repo_owner,
		repo.repo_name,
		commit_sha
	);

	let github_client =
		octorust::Client::new("patr", Credentials::Token(access_token.clone()))
			.map_err(|err| {
				log::info!("error while octorust init: {err:#}");
				err
			})
			.ok()
			.status(500)?;

	// update status of build to that commit in git_provider
	github_client
		.repos()
		.create_commit_status(
			&repo.repo_owner,
			&repo.repo_name,
			&commit_sha,
			&ReposCreateCommitStatusRequest {
				context: "patr-ci".to_owned(),
				state: status.commit_state(),
				description: status.description().to_owned(),
				// todo: how it works across different workspace
				target_url: format!(
					"https://app.patr.cloud/ci/github/{}/{}/build/{}",
					&repo.repo_owner, &repo.repo_name, build_num
				),
			},
		)
		.await
		.map_err(|err| {
			log::info!("error while updating ci repo commit status: {err:#}");
			err
		})
		.ok()
		.status(500)?;

	Ok(())
}
