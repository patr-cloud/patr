use api_models::{
	models::{
		ci::file_format::CiFlow,
		workspace::ci::git_provider::{BuildStatus, RepoStatus},
	},
	utils::Uuid,
};
use axum::{
	extract::{Path, State},
	response::Response,
};
use chrono::Utc;
use http::HeaderMap;

use crate::{
	db,
	models::ci::{
		github::CommitStatus,
		webhook_payload::github::Event,
		Commit,
		EventType,
		PullRequest,
		Tag,
	},
	prelude::*,
	rabbitmq::BuildId,
	service::{self, ParseStatus},
	utils::{constants::request_keys, Error},
};

pub async fn handle_ci_hooks_for_repo(
	Path(repo_id): Path<Uuid>,
	mut connection: Connection,
	State(config): State<Config>,
	headers: HeaderMap,
	body: String,
) -> Response {
	todo!("need to test registry webhook as compiler is not giving proper suggestions due to other errors");
	let request_id = Uuid::new_v4();

	log::trace!(
		"request_id: {request_id} - Processing ci webhook for repo {repo_id}"
	);

	// TODO: github is giving timeout status in webhooks settings for our
	// endpoint its better to process the payload in the message/event queue

	let event = match headers
		.get(request_keys::X_GITHUB_EVENT)
		.and_then(|e| e.to_str().ok())
	{
		Some(event) => event,
		None => {
			// not a known webhook header, send error
			return (StatusCode::BAD_GATEWAY, "Wrong Parameters");
		}
	};

	// handle github ping events
	if event.eq_ignore_ascii_case("ping") {
		return StatusCode::OK;
	}

	let repo = match db::get_repo_using_patr_repo_id(&mut connection, &repo_id)
		.await?
	{
		Some(repo) if repo.status == RepoStatus::Active => repo,
		_ => {
			log::trace!("request_id: {request_id} - ci not triggered, repo_id is either inactive or unknown");
			return (StatusCode::BAD_GATEWAY, "Wrong Parameters");
		}
	};

	// validate the payload data signature
	let Some(signature_in_header) = headers
	.get(request_keys::X_HUB_SIGNATURE_256)
	.and_then(|e| e.to_str()) else {
		return (StatusCode::BAD_GATEWAY, "Wrong Parameters");

	};

	repo.webhook_secret
		.and_then(|secret| {
			service::verify_github_payload_signature_256(
				&signature_in_header,
				&body.bytes(),
				&secret,
			)
			.ok()
		})
		.status(400)?;

	let event = serde_json::from_str(&body)?;

	let event_type = match event {
		Event::Push(pushed) => {
			if pushed.after == "0000000000000000000000000000000000000000" {
				// push event is triggered for delete branch and delete tag
				// with empty commit sha, skip those events
				return StatusCode::OK;
			}

			if let Some(branch_name) = pushed.ref_.strip_prefix("refs/heads/") {
				EventType::Commit(Commit {
					repo_owner: pushed.repository.owner.login,
					repo_name: pushed.repository.name,
					committed_branch_name: branch_name.to_string(),
					commit_sha: pushed.after,
					author: pushed
						.commits
						.first()
						.map(|commit| commit.author.name.clone()),
					commit_message: pushed
						.commits
						.first()
						.map(|commit| commit.message.clone()),
				})
			} else if let Some(tag_name) =
				pushed.ref_.strip_prefix("refs/tags/")
			{
				EventType::Tag(Tag {
					repo_owner: pushed.repository.owner.login,
					repo_name: pushed.repository.name,
					commit_sha: pushed.after,
					tag_name: tag_name.to_string(),
					author: pushed
						.commits
						.first()
						.map(|commit| commit.author.name.clone()),
					commit_message: pushed
						.commits
						.first()
						.map(|commit| commit.message.clone()),
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
				pr_repo_owner: pull_opened.pull_request.head.repo.owner.login,
				pr_repo_name: pull_opened.pull_request.head.repo.name,
				repo_owner: pull_opened.repository.owner.login,
				repo_name: pull_opened.repository.name,
				commit_sha: pull_opened.pull_request.head.sha,
				to_be_committed_branch_name: pull_opened.pull_request.base.ref_,
				pr_number: pull_opened.pull_request.number.to_string(),
				author: pull_opened.pull_request.head.user.name,
				pr_title: pull_opened.pull_request.title,
			})
		}
		Event::PullRequestSynchronize(pull_synced) => {
			EventType::PullRequest(PullRequest {
				pr_repo_owner: pull_synced.pull_request.head.repo.owner.login,
				pr_repo_name: pull_synced.pull_request.head.repo.name,
				repo_owner: pull_synced.repository.owner.login,
				repo_name: pull_synced.repository.name,
				to_be_committed_branch_name: pull_synced.pull_request.base.ref_,
				pr_number: pull_synced.pull_request.number.to_string(),
				commit_sha: pull_synced.pull_request.head.sha,
				author: pull_synced.pull_request.head.user.name,
				pr_title: pull_synced.pull_request.title,
			})
		}
	};

	let git_provider = db::get_git_provider_details_by_id(
		&mut connection,
		&repo.git_provider_id,
	)
	.await?
	.status(500)?;

	let access_token = git_provider.password.status(500)?;

	let ci_file_content = service::fetch_ci_file_content_from_github_repo(
		event_type.repo_owner(),
		event_type.repo_name(),
		event_type.commit_sha(),
		&access_token,
	)
	.await?;

	let build_num =
		service::create_build_for_repo(&mut connection, &repo.id, &event_type)
			.await?;

	let ci_flow = match service::parse_ci_file_content(
		&mut connection,
		&git_provider.workspace_id,
		&ci_file_content,
		&request_id,
	)
	.await?
	{
		ParseStatus::Success(ci_file) => ci_file,
		ParseStatus::Error(err) => {
			db::update_build_status(
				&mut connection,
				&repo.id,
				build_num,
				BuildStatus::Errored,
			)
			.await?;
			db::update_build_message(
				&mut connection,
				&repo.id,
				build_num,
				&err,
			)
			.await?;
			db::update_build_finished_time(
				&mut connection,
				&repo.id,
				build_num,
				&Utc::now(),
			)
			.await?;

			// returning success so that webhook msg will be ignored
			return StatusCode::OK;
		}
	};

	let CiFlow::Pipeline(pipeline) = ci_flow;
	let works = match service::evaluate_work_steps_for_ci(
		pipeline.steps,
		&event_type,
	) {
		Ok(works) => match works {
			service::EvaluationStatus::Success(works) => works,
			service::EvaluationStatus::Error(err) => {
				db::update_build_status(
					&mut connection,
					&repo.id,
					build_num,
					BuildStatus::Errored,
				)
				.await?;
				db::update_build_message(
					&mut connection,
					&repo.id,
					build_num,
					&err,
				)
				.await?;
				db::update_build_finished_time(
					&mut connection,
					&repo.id,
					build_num,
					&Utc::now(),
				)
				.await?;
				return StatusCode::OK;
			}
		},
		Err(err) => {
			log::info!("request_id: {request_id} - Error while evaluating ci work steps {err:#?}");
			db::update_build_status(
				&mut connection,
				&repo.id,
				build_num,
				BuildStatus::Errored,
			)
			.await?;
			db::update_build_finished_time(
				&mut connection,
				&repo.id,
				build_num,
				&Utc::now(),
			)
			.await?;
			// returning success so that webhook msg will be ignored
			return StatusCode::OK;
		}
	};

	service::add_build_steps_in_db(
		&mut connection,
		&repo.id,
		build_num,
		&works,
		&request_id,
	)
	.await?;

	connection.commit().await?;

	service::update_github_commit_status_for_build(
		&mut connection,
		&git_provider.workspace_id,
		&repo_id,
		build_num,
		CommitStatus::Running,
	)
	.await?;

	service::queue_check_and_start_ci_build(
		BuildId {
			repo_workspace_id: git_provider.workspace_id,
			repo_id: repo.id.clone(),
			build_num,
		},
		pipeline.services,
		works,
		event_type,
		&config,
		&request_id,
	)
	.await?;

	StatusCode::OK
}
