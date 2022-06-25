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

	let push_event = context.get_body_as::<github_types::PushEvent>()?;
	let (owner_name, repo_name) =
		push_event.repository.full_name.split_once('/').unwrap();
	let repo_url = push_event.repository.clone_url;
	let branch_name = push_event
		.git_ref
		.strip_prefix("refs/heads/")
		.expect("branch name has to be expected");

	let github_client = octorust::Client::new("patr", None).unwrap(); // TODO: use git credentials
	let ci_file = github_client
		.repos()
		.get_content_file(owner_name, repo_name, "patr.yml", branch_name)
		.await
		.unwrap();
	let ci_file = base64::decode(
		ci_file
			.content
			.chars()
			.filter(|ch| !ch.is_whitespace())
			.collect::<String>(),
	)?;

	let config = &context.get_state().config;
	let kube_client = service::get_kubernetes_config(config).await?;

	super::create_ci_pipeline(
		ci_file,
		&repo_url,
		repo_name,
		branch_name,
		kube_client,
	)
	.await?;

	Ok(())
}
