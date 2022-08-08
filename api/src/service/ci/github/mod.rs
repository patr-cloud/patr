use api_models::utils::Uuid;
use eve_rs::{AsError, Context};
use hmac::{Hmac, Mac};
use octorust::auth::Credentials;
use sha2::Sha256;

use self::payload_types::PushEvent;
use crate::{
	db::{self, Repository},
	utils::{Error, EveContext},
};

type HmacSha256 = Hmac<Sha256>;

pub mod payload_types;

pub const X_HUB_SIGNATURE_256: &str = "x-hub-signature-256";
pub const X_GITHUB_EVENT: &str = "x-github-event";

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

async fn find_matching_repo_with_secret(
	context: &mut EveContext,
) -> Result<Option<(Repository, PushEvent)>, Error> {
	let push_event = context.get_body_as::<PushEvent>()?;

	let signature_in_header = context
		.get_header(X_HUB_SIGNATURE_256)
		.status(400)
		.body("x-hub-signature-256 header not found")?;
	let payload = context.get_request().get_body_bytes().to_owned();

	let repo_list = db::get_repo_for_git_url(
		context.get_database_connection(),
		&push_event.repository.git_url,
	)
	.await?;

	for repo in repo_list {
		if verify_payload_signature(
			&signature_in_header,
			&payload,
			&repo.webhook_secret,
		)
		.await
		.is_ok()
		{
			return Ok(Some((repo, push_event)));
		}
	}

	Ok(None)
}

pub async fn ci_push_event(context: &mut EveContext) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	log::info!(
		"request_id: {request_id} - Processing github webhook payload..."
	);

	let (repo, push_event) = find_matching_repo_with_secret(context)
		.await?
		.status(400)
		.body("not a valid payload")?;

	let (owner_name, repo_name) = push_event
		.repository
		.full_name
		.rsplit_once('/')
		.status(400)
		.body("invalid repo name")?;
	let repo_clone_url = push_event.repository.clone_url;

	let access_token = db::get_access_token_for_repo(
		context.get_database_connection(),
		&repo.id,
	)
	.await?
	.status(500)
	.body("internal server error")?;

	let github_client =
		octorust::Client::new("patr", Credentials::Token(access_token.clone()))
			.map_err(|err| {
				log::info!("error while octorust init: {err:#}");
				err
			})
			.ok()
			.status(500)
			.body("error while initailizing octorust")?;

	let ci_file = github_client
		.repos()
		.get_content_file(owner_name, repo_name, "patr.yml", &push_event.after)
		.await
		.ok()
		.status(500)
		.body("patr.yml file is not defined")?;

	let ci_file = reqwest::Client::new()
		.get(ci_file.download_url)
		.bearer_auth(&access_token)
		.send()
		.await?
		.bytes()
		.await?;

	let _build_num = super::create_ci_build(
		context,
		&repo,
		&push_event.ref_,
		&push_event.after,
		ci_file,
		&access_token,
		&repo_clone_url,
		repo_name,
		request_id,
	)
	.await?;

	Ok(())
}
