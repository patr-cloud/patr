use eve_rs::AsError;
use hmac::{Hmac, Mac};
use octorust::auth::Credentials;
use sha2::Sha256;

use crate::utils::Error;

type HmacSha256 = Hmac<Sha256>;

pub mod payload_types;

pub const X_HUB_SIGNATURE_256: &str = "x-hub-signature-256";
pub const X_GITHUB_EVENT: &str = "x-github-event";

/// Returns error if payload signature is different from header signature
pub async fn verify_github_payload_signature_256(
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
