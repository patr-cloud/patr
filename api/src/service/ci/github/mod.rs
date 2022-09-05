use std::collections::HashMap;

use api_models::{
	models::workspace::ci::git_provider::BuildStatus,
	utils::Uuid,
};
use chrono::Utc;
use eve_rs::AsError;
use hmac::{Hmac, Mac};
use octorust::{
	auth::Credentials,
	types::{Order, ReposListOrgSort, ReposListVisibility},
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
	let (git_ref, commit_sha) = match &event_type {
		EventType::Commit(commit) => (
			format!("refs/heads/{}", commit.committed_branch_name),
			&commit.commit_sha,
		),
		EventType::Tag(tag) => {
			(format!("refs/tags/{}", tag.tag_name), &tag.commit_sha)
		}
		EventType::PullRequest(pull_request) => (
			format!("refs/pull/{}", pull_request.pr_number),
			&pull_request.commit_sha,
		),
	};

	let build_num = db::generate_new_build_for_repo(
		connection,
		repo_id,
		&git_ref,
		commit_sha,
		BuildStatus::Running,
		&Utc::now(),
	)
	.await?;

	Ok(build_num)
}

pub async fn fetch_ci_file_content_from_github_repo_based_on_event(
	event_type: &EventType,
	access_token: &str,
) -> Result<Vec<u8>, Error> {
	// fetch ci file from remote repo for forked pull requests
	let (owner_name, repo_name, commit_sha) = match event_type {
		EventType::Commit(commit) => {
			(&commit.repo_owner, &commit.repo_name, &commit.commit_sha)
		}
		EventType::Tag(tag) => {
			(&tag.repo_owner, &tag.repo_name, &tag.commit_sha)
		}
		EventType::PullRequest(pull_request) => (
			&pull_request.head_repo_owner,
			&pull_request.head_repo_name,
			&pull_request.commit_sha,
		),
	};

	fetch_ci_file_content_from_github_repo(
		owner_name,
		repo_name,
		access_token,
		commit_sha,
	)
	.await
}

pub async fn fetch_ci_file_content_from_github_repo(
	owner_name: &str,
	repo_name: &str,
	access_token: &str,
	git_ref: &str, // name of the commit/branch/tag
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
