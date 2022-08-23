use std::collections::HashMap;

use api_models::utils::Uuid;
use eve_rs::AsError;
use hmac::{Hmac, Mac};
use octorust::{
	auth::Credentials,
	types::{Order, ReposListOrgSort, ReposListVisibility},
};
use sha2::Sha256;

use super::MutableRepoValues;
use crate::{db, service, utils::Error, Database};

type HmacSha256 = Hmac<Sha256>;

pub mod payload_types;

pub const X_HUB_SIGNATURE_256: &str = "x-hub-signature-256";
pub const X_GITHUB_EVENT: &str = "x-github-event";

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

pub async fn fetch_ci_file_content_from_github_repo(
	owner_name: &str,
	repo_name: &str,
	access_token: &str,
	git_commit: &str,
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
		.get_content_file(owner_name, repo_name, "patr.yml", git_commit)
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
		git_provider_id,
		repos_in_github,
		repos_in_db,
	)
	.await?;

	Ok(())
}
