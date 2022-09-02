use api_models::{
	models::{
		ci::file_format::CiFlow,
		workspace::ci::git_provider::{BuildStatus, RepoStatus},
	},
	utils::Uuid,
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::ci::{
		webhook_payload::github::Event,
		Commit,
		EventType,
		PullRequest,
		Tag,
	},
	pin_fn,
	rabbitmq::BuildId,
	service::{self, Netrc, ParseStatus},
	utils::{
		constants::request_keys,
		Error,
		ErrorData,
		EveContext,
		EveMiddleware,
	},
};

pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut sub_app = create_eve_app(app);

	sub_app.post(
		"/repo/:repoId",
		[EveMiddleware::CustomFunction(pin_fn!(
			handle_ci_hooks_for_repo
		))],
	);

	sub_app
}

async fn handle_ci_hooks_for_repo(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let repo_id =
		Uuid::parse_str(context.get_param(request_keys::REPO_ID).unwrap())
			.unwrap();

	log::trace!(
		"request_id: {request_id} - Processing ci webhook for repo {repo_id}"
	);

	// TODO: github is giving timeout status in webhooks settings for our
	// endpoint its better to process the payload in the message/event queue

	let event = match context.get_header(request_keys::X_GITHUB_EVENT) {
		Some(event) => event,
		None => {
			// not a known webhook header, send error
			return Err(Error::empty()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string()));
		}
	};

	// handle github ping events
	if event.eq_ignore_ascii_case("ping") {
		return Ok(context);
	}

	let repo = match db::get_repo_using_patr_repo_id(
		context.get_database_connection(),
		&repo_id,
	)
	.await?
	{
		Some(repo) if repo.status == RepoStatus::Active => repo,
		_ => {
			log::trace!("request_id: {request_id} - ci not triggered, repo_id is either inactive or unknown");
			return Err(Error::empty()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string()));
		}
	};

	// validate the payload data signature
	let signature_in_header = context
		.get_header(request_keys::X_HUB_SIGNATURE_256)
		.status(400)?;

	repo.webhook_secret
		.and_then(|secret| {
			service::verify_github_payload_signature_256(
				&signature_in_header,
				&context.get_request().get_body_bytes(),
				&secret,
			)
			.ok()
		})
		.status(400)?;

	let event = context.get_body_as::<Event>()?;

	let event_type = match event {
		Event::Push(pushed) => {
			if pushed.after == "0000000000000000000000000000000000000000" {
				// push event is triggered for delete branch and delete tag
				// with empty commit sha, skip those events
				return Ok(context);
			}

			if let Some(branch_name) = pushed.ref_.strip_prefix("refs/heads/") {
				EventType::Commit(Commit {
					repo_owner: pushed.repository.owner.login,
					repo_name: pushed.repository.name,
					committed_branch_name: branch_name.to_string(),
					commit_sha: pushed.after,
				})
			} else if let Some(tag_name) =
				pushed.ref_.strip_prefix("refs/tags/")
			{
				EventType::Tag(Tag {
					repo_owner: pushed.repository.owner.login,
					repo_name: pushed.repository.name,
					commit_sha: pushed.after,
					tag_name: tag_name.to_string(),
				})
			} else {
				log::trace!(
					"request_id: {request_id} - Error while parsing ref {}",
					pushed.ref_
				);
				return Error::as_result().status(500)?;
			}
		}
		Event::PullRequestOpened(pull_opened) => {
			EventType::PullRequest(PullRequest {
				head_repo_owner: pull_opened.pull_request.head.repo.owner.login,
				head_repo_name: pull_opened.pull_request.head.repo.name,
				commit_sha: pull_opened.pull_request.head.sha,
				to_be_committed_branch_name: pull_opened.pull_request.base.ref_,
				pr_number: pull_opened.pull_request.number.to_string(),
			})
		}
		Event::PullRequestSynchronize(pull_synced) => {
			EventType::PullRequest(PullRequest {
				head_repo_owner: pull_synced.pull_request.head.repo.owner.login,
				head_repo_name: pull_synced.pull_request.head.repo.name,
				to_be_committed_branch_name: pull_synced.pull_request.base.ref_,
				pr_number: pull_synced.pull_request.number.to_string(),
				commit_sha: pull_synced.pull_request.head.sha,
			})
		}
	};

	let git_provider = db::get_git_provider_details_by_id(
		context.get_database_connection(),
		&repo.git_provider_id,
	)
	.await?
	.status(500)?;

	let (login_name, access_token) = git_provider
		.login_name
		.zip(git_provider.password)
		.status(500)?;

	let ci_file_content =
		service::fetch_ci_file_content_from_github_repo_based_on_event(
			&event_type,
			&access_token,
		)
		.await?;

	let build_num = service::create_build_for_repo(
		context.get_database_connection(),
		&repo.id,
		&event_type,
	)
	.await?;

	let ci_flow = match service::parse_ci_file_content(
		context.get_database_connection(),
		&git_provider.workspace_id,
		&ci_file_content,
		&request_id,
	)
	.await?
	{
		ParseStatus::Success(ci_file) => ci_file,
		ParseStatus::Error => {
			db::update_build_status(
				context.get_database_connection(),
				&repo.id,
				build_num,
				BuildStatus::Errored,
			)
			.await?;
			return Ok(context);
		}
	};

	let CiFlow::Pipeline(pipeline) = ci_flow;
	let works = match service::evaluate_work_steps_for_ci(
		pipeline.steps,
		&event_type,
	) {
		Ok(works) => works,
		Err(err) => {
			log::info!("request_id: {request_id} - Error while evaluating ci work steps {err:#?}");
			db::update_build_status(
				context.get_database_connection(),
				&repo.id,
				build_num,
				BuildStatus::Errored,
			)
			.await?;
			return Ok(context);
		}
	};

	service::add_build_steps_in_db(
		context.get_database_connection(),
		&repo.id,
		build_num,
		&works,
		&request_id,
	)
	.await?;

	context.commit_database_transaction().await?;

	let config = context.get_state().config.clone();
	service::add_build_steps_in_k8s(
		context.get_database_connection(),
		&config,
		&repo.id,
		&repo.repo_name,
		&BuildId {
			repo_workspace_id: git_provider.workspace_id,
			repo_id: repo.id.clone(),
			build_num,
		},
		pipeline.services,
		works,
		Some(Netrc {
			machine: "github.com".to_owned(),
			login: login_name,
			password: access_token,
		}),
		event_type,
		&repo.clone_url,
		&request_id,
	)
	.await?;

	Ok(context)
}
