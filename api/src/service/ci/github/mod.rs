use std::collections::{HashMap, HashSet};

use api_models::{
	models::workspace::ci2::github::{BuildStatus, BuildStepStatus},
	utils::Uuid,
};
use chrono::Utc;
use eve_rs::{AsError, Context};
use hmac::{Hmac, Mac};
use octorust::auth::Credentials;
use sha2::Sha256;

use self::payload_types::PushEvent;
use super::Netrc;
use crate::{
	db::{self, Repository},
	models::ci::file_format::{CiFlow, EnvVarValue, Step},
	rabbitmq::BuildId,
	utils::{Error, EveContext},
	Database,
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
	let branch_name = push_event
		.ref_
		.strip_prefix("refs/heads/")
		.status(500)
		.body("currently only push on branches is supported")?;

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
		.get_content_file(owner_name, repo_name, "patr.yml", branch_name)
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

	let build_num = db::generate_new_build_for_repo(
		context.get_database_connection(),
		&repo.id,
		&push_event.ref_,
		&push_event.after,
		BuildStatus::Running,
		&Utc::now(),
	)
	.await?;

	let mut ci_flow: CiFlow = serde_yaml::from_slice(ci_file.as_ref())?;
	let CiFlow::Pipeline(pipeline) = ci_flow.clone();

	// validate the ci file
	if !is_names_unique(&ci_flow) {
		log::info!(
			"request_id: {request_id} - Invalid ci config file, marking build as errored"
		);
		db::update_build_status(
			context.get_database_connection(),
			&repo.id,
			build_num,
			BuildStatus::Errored,
		)
		.await?;

		return Ok(());
	}

	if !find_and_replace_secret_names(
		context.get_database_connection(),
		&mut ci_flow,
		&repo.workspace_id,
	)
	.await?
	{
		log::info!(
			"request_id: {request_id} - Invalid secret name given, marking build as errored"
		);
		db::update_build_status(
			context.get_database_connection(),
			&repo.id,
			build_num,
			BuildStatus::Errored,
		)
		.await?;

		return Ok(());
	};

	// add cloning as a step
	db::add_ci_steps_for_build(
		context.get_database_connection(),
		&repo.id,
		build_num,
		0,
		"git-clone",
		"",
		vec![],
		BuildStepStatus::WaitingToStart,
	)
	.await?;

	for (
		step_count,
		Step {
			name,
			image,
			commands,
			env: _,
		},
	) in pipeline.steps.into_iter().enumerate()
	{
		db::add_ci_steps_for_build(
			context.get_database_connection(),
			&repo.id,
			build_num,
			step_count as i32 + 1,
			&name,
			&image,
			vec![commands.to_string()],
			BuildStepStatus::WaitingToStart,
		)
		.await?;
	}

	context.commit_database_transaction().await?;

	// TODO: make more generic
	let netrc = Netrc {
		machine: "github.com".to_string(),
		login: "oauth".to_string(),
		password: access_token,
	};

	super::create_ci_pipeline(
		ci_flow,
		&repo_clone_url,
		repo_name,
		branch_name,
		Some(netrc),
		BuildId {
			workspace_id: repo.workspace_id,
			repo_id: repo.id,
			build_num,
		},
		&context.get_state().config.clone(),
		context.get_database_connection(),
		&request_id,
	)
	.await?;

	Ok(())
}

fn is_names_unique(ci_flow: &CiFlow) -> bool {
	let CiFlow::Pipeline(pipeline) = ci_flow;

	let mut step_names = HashSet::new();
	for step in &pipeline.steps {
		if !step_names.insert(step.name.as_str()) {
			return false;
		}
	}

	let mut service_names = HashSet::new();
	for service in &pipeline.services {
		if !service_names.insert(service.name.as_str()) {
			return false;
		}
	}

	true
}

async fn find_and_replace_secret_names(
	connection: &mut <Database as sqlx::Database>::Connection,
	ci_flow: &mut CiFlow,
	workspace_id: &Uuid,
) -> Result<bool, Error> {
	let workspace_secrets =
		db::get_all_secrets_in_workspace(connection, workspace_id)
			.await?
			.into_iter()
			.map(|secret| (secret.name, secret.id))
			.collect::<HashMap<_, _>>();

	let CiFlow::Pipeline(pipeline) = ci_flow;

	for service in &mut pipeline.services {
		for env in &mut service.env {
			if let EnvVarValue::ValueFromSecret(secret_name) = &mut env.value {
				if let Some(secret_id) = workspace_secrets.get(&*secret_name) {
					*secret_name = secret_id.to_string();
				} else {
					return Ok(false);
				}
			}
		}
	}

	for step in &mut pipeline.steps {
		for env in &mut step.env {
			if let EnvVarValue::ValueFromSecret(secret_name) = &mut env.value {
				if let Some(secret_id) = workspace_secrets.get(&*secret_name) {
					*secret_name = secret_id.to_string();
				} else {
					return Ok(false);
				}
			}
		}
	}

	Ok(true)
}
