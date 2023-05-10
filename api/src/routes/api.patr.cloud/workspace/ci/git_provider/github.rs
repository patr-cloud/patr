use api_models::{
	models::{ci::file_format::CiFlow, prelude::*},
	utils::{Base64String, Uuid},
};
use axum::{extract::State, Router};
use chrono::Utc;
use octorust::{
	self,
	auth::Credentials,
	types::{ReposCreateWebhookRequest, ReposCreateWebhookRequestConfig},
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use redis::AsyncCommands;
use serde::Deserialize;

use crate::{
	app::App,
	db,
	error,
	models::{
		ci::{github::CommitStatus, Commit, EventType, PullRequest, Tag},
		deployment::Logs,
		rbac::permissions,
	},
	prelude::*,
	rabbitmq::{BuildId, BuildStepId},
	service::{self, ParseStatus},
	utils::{constants::request_keys, Error},
};

pub fn create_sub_app(app: &App) -> Router<App> {
	Router::new()
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::ci::git_provider::CONNECT,
				|GithubAuthPath { workspace_id }, (), app, request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			),
			app.clone(),
			connect_to_github,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::ci::git_provider::CONNECT,
				|GithubAuthCallbackPath { workspace_id }, (), app, request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			),
			app.clone(),
			github_oauth_callback,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::ci::git_provider::LIST,
				|SyncReposPath { workspace_id }, (), app, request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			),
			app.clone(),
			sync_repositories,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::ci::git_provider::repo::LIST,
				|ListReposPath { workspace_id }, (), app, request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			),
			app.clone(),
			list_repositories,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::ci::git_provider::repo::ACTIVATE,
				|GetRepoInfoPath {
				     workspace_id,
				     repo_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					let repo =
						db::get_repo_details_using_github_uid_for_workspace(
							&mut connection,
							&workspace_id,
							&repo_id,
						)
						.await?
						.ok_or_else(|| ErrorType::internal_error())?;

					db::get_resource_by_id(&mut connection, &repo.id).await
				},
			),
			app.clone(),
			get_repo_info,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::ci::git_provider::repo::ACTIVATE,
				|ActivateRepoPath {
				     workspace_id,
				     repo_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					let repo =
						db::get_repo_details_using_github_uid_for_workspace(
							&mut connection,
							&workspace_id,
							&repo_id,
						)
						.await?
						.ok_or_else(|| ErrorType::internal_error())?;

					db::get_resource_by_id(&mut connection, &repo.id).await
				},
			),
			app.clone(),
			activate_repo,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::ci::git_provider::repo::build::LIST,
				|GetBuildListPath {
				     workspace_id,
				     repo_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					let repo =
						db::get_repo_details_using_github_uid_for_workspace(
							&mut connection,
							&workspace_id,
							&repo_id,
						)
						.await?
						.ok_or_else(|| ErrorType::internal_error())?;

					db::get_resource_by_id(&mut connection, &repo.id).await
				},
			),
			app.clone(),
			get_build_list,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::ci::git_provider::repo::build::INFO,
				|GetBuildInfoPath {
				     workspace_id,
				     repo_id,
				     ..
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					let repo =
						db::get_repo_details_using_github_uid_for_workspace(
							&mut connection,
							&workspace_id,
							&repo_id,
						)
						.await?
						.ok_or_else(|| ErrorType::internal_error())?;

					db::get_resource_by_id(&mut connection, &repo.id).await
				},
			),
			app.clone(),
			get_build_info,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::ci::git_provider::repo::build::INFO,
				|GetBuildLogPath {
				     workspace_id,
				     repo_id,
				     ..
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					let repo =
						db::get_repo_details_using_github_uid_for_workspace(
							&mut connection,
							&workspace_id,
							&repo_id,
						)
						.await?
						.ok_or_else(|| ErrorType::internal_error())?;

					db::get_resource_by_id(&mut connection, &repo.id).await
				},
			),
			app.clone(),
			get_build_logs,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::ci::git_provider::repo::build::CANCEL,
				|CancelBuildPath {
				     workspace_id,
				     repo_id,
				     ..
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					let repo =
						db::get_repo_details_using_github_uid_for_workspace(
							&mut connection,
							&workspace_id,
							&repo_id,
						)
						.await?
						.ok_or_else(|| ErrorType::internal_error())?;

					db::get_resource_by_id(&mut connection, &repo.id).await
				},
			),
			app.clone(),
			cancel_build,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::ci::git_provider::repo::build::RESTART,
				|RestartBuildPath {
				     workspace_id,
				     repo_id,
				     ..
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					let repo =
						db::get_repo_details_using_github_uid_for_workspace(
							&mut connection,
							&workspace_id,
							&repo_id,
						)
						.await?
						.ok_or_else(|| ErrorType::internal_error())?;

					db::get_resource_by_id(&mut connection, &repo.id).await
				},
			),
			app.clone(),
			restart_build,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::ci::git_provider::repo::build::START,
				|StartBuildPath {
				     workspace_id,
				     repo_id,
				     ..
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					let repo =
						db::get_repo_details_using_github_uid_for_workspace(
							&mut connection,
							&workspace_id,
							&repo_id,
						)
						.await?
						.ok_or_else(|| ErrorType::internal_error())?;

					db::get_resource_by_id(&mut connection, &repo.id).await
				},
			),
			app.clone(),
			start_build_for_branch,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::ci::git_provider::repo::LIST,
				|ListGitRefForRepoPath {
				     workspace_id,
				     repo_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					let repo =
						db::get_repo_details_using_github_uid_for_workspace(
							&mut connection,
							&workspace_id,
							&repo_id,
						)
						.await?
						.ok_or_else(|| ErrorType::internal_error())?;

					db::get_resource_by_id(&mut connection, &repo.id).await
				},
			),
			app.clone(),
			list_git_ref_for_repo,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::ci::git_provider::repo::INFO,
				|GetPatrCiFilePath {
				     workspace_id,
				     repo_id,
				     ..
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					let repo =
						db::get_repo_details_using_github_uid_for_workspace(
							&mut connection,
							&workspace_id,
							&repo_id,
						)
						.await?
						.ok_or_else(|| ErrorType::internal_error())?;

					db::get_resource_by_id(&mut connection, &repo.id).await
				},
			),
			app.clone(),
			get_patr_ci_file,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::ci::git_provider::LIST,
				|WritePatrCiFilePath {
				     workspace_id,
				     repo_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					let repo =
						db::get_repo_details_using_github_uid_for_workspace(
							&mut connection,
							&workspace_id,
							&repo_id,
						)
						.await?
						.ok_or_else(|| ErrorType::internal_error())?;

					db::get_resource_by_id(&mut connection, &repo.id).await
				},
			),
			app.clone(),
			write_patr_ci_file,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::ci::git_provider::DISCONNECT,
				|GithubSignOutPath { workspace_id }, (), app, request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			),
			app.clone(),
			sign_out,
		)
}

async fn connect_to_github(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: GithubAuthPath { workspace_id },
		query: (),
		body: (),
	}: DecodedRequest<GithubAuthRequest>,
) -> Result<GithubAuthResponse, Error> {
	let client_id = config.github.client_id.to_owned();

	// https://docs.github.com/en/developers/apps/building-oauth-apps/scopes-for-oauth-apps
	let scope = "repo";

	// https://docs.github.com/en/developers/apps/building-oauth-apps/authorizing-oauth-apps#2-users-are-redirected-back-to-your-site-by-github
	let state = thread_rng()
		.sample_iter(&Alphanumeric)
		.take(8)
		.map(char::from)
		.collect::<String>();

	// temporary code from github will expire within 10 mins,
	// so we are using extra 5 mins as buffer time for now
	let ttl_in_secs = 15 * 60; // 15 mins
	let value = workspace_id.as_str();
	config
		.redis
		.set_ex(format!("githubAuthState:{state}"), value, ttl_in_secs)
		.await?;

	let oauth_url = format!("https://github.com/login/oauth/authorize?client_id={client_id}&scope={scope}&state={state}");
	Ok(GithubAuthResponse { oauth_url })
}

async fn github_oauth_callback(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: GithubAuthCallbackPath { workspace_id },
		query: (),
		body: GithubAuthCallbackRequest { code, state },
	}: DecodedRequest<GithubAuthCallbackRequest>,
) -> Result<(), Error> {
	// validate the state value
	let expected_value = workspace_id.as_str();
	let value_from_redis: Option<String> =
		config.redis.get(format!("githubAuthState:{state}")).await?;
	if !value_from_redis
		.map(|value| value == expected_value)
		.unwrap_or(false)
	{
		return Err(ErrorType::InvalidStateValue);
	}

	#[derive(Deserialize)]
	struct GitHubAccessTokenResponse {
		access_token: String,
		scope: String,
		token_type: String,
	}

	let GitHubAccessTokenResponse {
		access_token,
		scope: _scope,
		token_type: _token_type,
	} = reqwest::Client::builder()
		.build()?
		.post("https://github.com/login/oauth/access_token")
		.query(&[
			("client_id", config.github.client_id.clone()),
			("client_secret", config.github.client_secret.clone()),
			("code", code),
		])
		.header("accept", "application/json")
		.send()
		.await?
		.error_for_status()?
		.json::<GitHubAccessTokenResponse>()
		.await?;

	let github_client =
		octorust::Client::new("patr", Credentials::Token(access_token.clone()))
			.map_err(|err| {
				log::info!("error while octorust init: {err:#}");
				err
			})
			.ok()
			.ok_or_else(|| ErrorType::internal_error())?;

	let login_name = github_client
		.users()
		.get_authenticated_public_user()
		.await
		.map_err(|err| {
			log::info!("error while getting login name: {err:#}");
			err
		})
		.ok()
		.ok_or_else(|| ErrorType::internal_error())?
		.login;

	db::add_git_provider_to_workspace(
		&mut connection,
		&workspace_id,
		"github.com",
		GitProviderType::Github,
		Some(&login_name),
		Some(&access_token),
	)
	.await?;

	Ok(())
}

async fn sync_repositories(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: SyncReposPath { workspace_id },
		query: (),
		body: (),
	}: DecodedRequest<SyncReposRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {request_id} - Syncing github repos for workspace {workspace_id}");

	let git_provider = db::get_git_provider_details_for_workspace_using_domain(
		&mut connection,
		&workspace_id,
		"github.com",
	)
	.await?
	.ok_or_else(|| ErrorType::internal_error())?;

	let (_login_name, access_token) = git_provider
		.login_name
		.zip(git_provider.password)
		.ok_or_else(|| ErrorType::internal_error())?;

	if !git_provider.is_syncing {
		db::set_syncing(&mut connection, &git_provider.id, true, None).await?;
		service::queue_sync_github_repo(
			&git_provider.workspace_id,
			&git_provider.id,
			&request_id,
			access_token,
			&config,
		)
		.await?;
	}

	Ok(());
}

async fn list_repositories(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: ListReposPath { workspace_id },
		query: (),
		body: (),
	}: DecodedRequest<ListReposRequest>,
) -> Result<ListReposResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {request_id} - Listing github repos for workspace {workspace_id}");

	let git_provider = db::get_git_provider_details_for_workspace_using_domain(
		&mut connection,
		&workspace_id,
		"github.com",
	)
	.await?
	.ok_or_else(|| ErrorType::internal_error())?;

	let repos =
		db::list_repos_for_git_provider(&mut connection, &git_provider.id)
			.await?
			.into_iter()
			.map(|repo| RepositoryDetails {
				id: repo.git_provider_repo_uid,
				name: repo.repo_name,
				repo_owner: repo.repo_owner,
				clone_url: repo.clone_url,
				status: repo.status,
				runner_id: repo.runner_id,
			})
			.collect();

	Ok(ListReposResponse { repos })
}

async fn get_repo_info(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: GetRepoInfoPath {
			workspace_id,
			repo_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<GetRepoInfoRequest>,
) -> Result<GetRepoInfoResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {request_id} - Activating CI for repo {repo_id}");

	let repo = db::get_repo_details_using_github_uid_for_workspace(
		&mut connection,
		&workspace_id,
		&repo_id,
	)
	.await?
	.ok_or_else(|| ErrorType::internal_error())?;

	Ok(GetRepoInfoResponse {
		repo: RepositoryDetails {
			id: repo.git_provider_repo_uid,
			name: repo.repo_name,
			repo_owner: repo.repo_owner,
			clone_url: repo.clone_url,
			status: repo.status,
			runner_id: repo.runner_id,
		},
	})
}

async fn activate_repo(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: ActivateRepoPath {
			workspace_id,
			repo_id,
		},
		query: (),
		body: ActivateRepoRequest { runner_id },
	}: DecodedRequest<ActivateRepoRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {request_id} - Activating CI for repo {repo_id}");

	let repo = db::get_repo_details_using_github_uid_for_workspace(
		&mut connection,
		&workspace_id,
		&repo_id,
	)
	.await?
	.ok_or_else(|| ErrorType::internal_error())?;

	let git_provider = db::get_git_provider_details_by_id(
		&mut connection,
		&repo.git_provider_id,
	)
	.await?
	.ok_or_else(|| ErrorType::internal_error())?;

	let (_login_name, access_token) = git_provider
		.login_name
		.zip(git_provider.password)
		.ok_or_else(|| ErrorType::internal_error())?;

	let is_valid_runner =
		db::get_runners_for_workspace(&mut connection, &workspace_id)
			.await?
			.into_iter()
			.any(|runner| runner.id == runner_id);

	if !is_valid_runner {
		return Err(ErrorType::WrongParameters);
	}

	let github_client =
		octorust::Client::new("patr", Credentials::Token(access_token))
			.map_err(|err| {
				log::info!("error while octorust init: {err:#}");
				err
			})
			.ok()
			.ok_or_else(|| ErrorType::internal_error())?;

	let webhook_secret =
		db::activate_ci_for_repo(&mut connection, &repo.id, &runner_id).await?;

	let _configured_webhook = github_client
		.repos()
		.create_webhook(
			&repo.repo_owner,
			&repo.repo_name,
			&ReposCreateWebhookRequest {
				active: Some(true),
				config: Some(ReposCreateWebhookRequestConfig {
					content_type: "json".to_string(),
					digest: "".to_string(),
					insecure_ssl: None,
					secret: webhook_secret,
					token: "".to_string(),
					url: service::get_webhook_url_for_repo(
						&config.api_url,
						&repo.id,
					),
				}),
				events: vec!["push".to_string(), "pull_request".to_string()],
				name: "web".to_string(),
			},
		)
		.await
		.map_err(|err| {
			log::info!("error while configuring webhooks: {err:#}");
			err
		})
		.ok()
		.ok_or_else(|| ErrorType::internal_error())?;

	Ok(())
}

async fn deactivate_repo(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: DeactivateRepoPath {
			workspace_id,
			repo_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<DeactivateRepoRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();

	log::trace!(
		"request_id: {request_id} - Deactivating CI for repo {repo_id}"
	);

	let repo = db::get_repo_details_using_github_uid_for_workspace(
		&mut connection,
		&workspace_id,
		&repo_id,
	)
	.await?
	.ok_or_else(|| ErrorType::internal_error())?;

	let git_provider = db::get_git_provider_details_by_id(
		&mut connection,
		&repo.git_provider_id,
	)
	.await?
	.ok_or_else(|| ErrorType::internal_error())?;
	let (_login_name, access_token) = git_provider
		.login_name
		.zip(git_provider.password)
		.ok_or_else(|| ErrorType::internal_error())?;

	let github_client =
		octorust::Client::new("patr", Credentials::Token(access_token))
			.map_err(|err| {
				log::info!("error while octorust init: {err:#}");
				err
			})
			.ok()
			.ok_or_else(|| ErrorType::internal_error())?;

	db::update_repo_status(
		&mut connection,
		&repo.git_provider_id,
		&repo.git_provider_repo_uid,
		RepoStatus::Inactive,
	)
	.await?;

	let all_webhooks = github_client
		.repos()
		.list_all_webhooks(&repo.repo_owner, &repo.repo_name)
		.await
		.map_err(|err| {
			log::info!("error while getting webhooks list: {err:#}");
			err
		})
		.ok()
		.ok_or_else(|| ErrorType::internal_error())?;

	let github_webhook_url =
		service::get_webhook_url_for_repo(&config.api_url, &repo.id);

	for webhook in all_webhooks {
		if webhook.config.url == github_webhook_url {
			github_client
				.repos()
				.delete_webhook(&repo.repo_owner, &repo.repo_name, webhook.id)
				.await
				.map_err(|err| {
					log::info!("error while deleting webhook: {err:#}");
					err
				})
				.ok()
				.ok_or_else(|| ErrorType::internal_error())?;
		}
	}

	Ok(())
}

async fn get_build_list(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: GetBuildListPath {
			workspace_id,
			repo_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<GetBuildListRequest>,
) -> Result<GetBuildListResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!(
		"request_id: {request_id} - Getting build list for repo {repo_id}"
	);

	let repo = db::get_repo_details_using_github_uid_for_workspace(
		&mut connection,
		&workspace_id,
		&repo_id,
	)
	.await?
	.ok_or_else(|| ErrorType::internal_error())?;

	let builds =
		db::list_build_details_for_repo(&mut connection, &repo.id).await?;

	Ok(GetBuildListResponse { builds });
}

async fn get_build_info(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: GetBuildInfoPath {
			workspace_id,
			repo_id,
			build_num,
		},
		query: (),
		body: (),
	}: DecodedRequest<GetBuildInfoRequest>,
) -> Result<GetBuildInfoResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {request_id} - Getting build info for repo {repo_id} - {build_num}");

	let repo = db::get_repo_details_using_github_uid_for_workspace(
		&mut connection,
		&workspace_id,
		&repo_id,
	)
	.await?
	.ok_or_else(|| ErrorType::internal_error())?;

	let build_info =
		db::get_build_details_for_build(&mut connection, &repo.id, build_num)
			.await?
			.ok_or_else(|| ErrorType::NotFound)?;

	let steps =
		db::list_build_steps_for_build(&mut connection, &repo.id, build_num)
			.await?;

	Ok(GetBuildInfoResponse { build_info, steps });
}

async fn get_build_logs(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path:
			GetBuildLogPath {
				workspace_id,
				repo_id,
				build_num,
				step,
			},
		query: (),
		body: (),
	}: DecodedRequest<GetBuildLogRequest>,
) -> Result<GetBuildLogResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {request_id} - Getting build logs for repo {repo_id} - {build_num} - {step}");

	let repo = db::get_repo_details_using_github_uid_for_workspace(
		&mut connection,
		&workspace_id,
		&repo_id,
	)
	.await?
	.ok_or_else(|| ErrorType::internal_error())?;

	let build_created_time =
		db::get_build_created_time(&mut connection, &repo.id, build_num)
			.await?
			.ok_or_else(|| ErrorType::internal_error())?;

	let build_step_id = BuildStepId {
		build_id: BuildId {
			repo_workspace_id: workspace_id,
			repo_id: repo.id,
			build_num,
		},
		step_id: step,
	};

	let loki = config.loki.clone();
	let response = reqwest::Client::new()
		.get(format!(
			"https://{}/loki/api/v1/query_range?query={{namespace=\"{}\",job=\"{}/{}\"}}&start={}",
			loki.host,
			build_step_id.build_id.get_build_namespace(),
			build_step_id.build_id.get_build_namespace(),
			build_step_id.get_job_name(),
			build_created_time.timestamp_nanos()
		))
		.basic_auth(&loki.username, Some(&loki.password))
		.send()
		.await?
		.json::<Logs>()
		.await?
		.data
		.result;

	let logs = response
		.into_iter()
		.flat_map(|loki_log| {
			loki_log.values.into_iter().map(|log| {
				let mut log = log.into_iter();
				(log.next(), log.next())
			})
		})
		.filter_map(|(time, log_msg)| {
			let original_log_time: u64 = time?.parse().ok()?;
			Some(BuildLogs {
				time: original_log_time
					.saturating_sub(build_created_time.timestamp_nanos() as u64),
				log: log_msg?,
			})
		})
		.collect();

	Ok(GetBuildLogResponse { logs });
}

async fn cancel_build(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: CancelBuildPath {
			workspace_id,
			repo_id,
			build_num,
		},
		query: (),
		body: (),
	}: DecodedRequest<CancelBuildRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {request_id} - Stopping build for repo {repo_id} - {build_num}");

	let repo = db::get_repo_details_using_github_uid_for_workspace(
		&mut connection,
		&workspace_id,
		&repo_id,
	)
	.await?
	.ok_or_else(|| ErrorType::internal_error())?;

	// get the build status with lock, so that it won't be updated in rabbitmq
	// until this route ends.
	let build_status =
		db::get_build_status_for_update(&mut connection, &repo.id, build_num)
			.await?
			.ok_or_else(|| ErrorType::NotFound)?;

	if build_status == BuildStatus::Running ||
		build_status == BuildStatus::WaitingToStart
	{
		db::update_build_status(
			&mut connection,
			&repo.id,
			build_num,
			BuildStatus::Cancelled,
		)
		.await?;
		db::update_build_finished_time(
			&mut connection,
			&repo.id,
			build_num,
			&Utc::now(),
		)
		.await?;

		service::update_github_commit_status_for_build(
			&mut connection,
			&workspace_id,
			&repo.id,
			build_num,
			CommitStatus::Errored,
		)
		.await?;
	}

	Ok(());
}

async fn restart_build(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: RestartBuildPath {
			workspace_id,
			repo_id,
			build_num,
		},
		query: (),
		body: (),
	}: DecodedRequest<RestartBuildRequest>,
) -> Result<RestartBuildResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {request_id} - Restarting build for repo {repo_id} - {build_num}");

	let repo = db::get_repo_details_using_github_uid_for_workspace(
		&mut connection,
		&workspace_id,
		&repo_id,
	)
	.await?
	.ok_or_else(|| ErrorType::internal_error())?;

	let git_provider = db::get_git_provider_details_by_id(
		&mut connection,
		&repo.git_provider_id,
	)
	.await?
	.ok_or_else(|| ErrorType::internal_error())?;

	let access_token = git_provider
		.password
		.ok_or_else(|| ErrorType::internal_error())?;

	let previous_build =
		db::get_build_details_for_build(&mut connection, &repo.id, build_num)
			.await?
			.ok_or_else(|| ErrorType::internal_error())?;

	let event_type = if let Some(branch_name) =
		previous_build.git_ref.strip_prefix("refs/heads/")
	{
		EventType::Commit(Commit {
			repo_owner: repo.repo_owner,
			repo_name: repo.repo_name.clone(),
			commit_sha: previous_build.git_commit,
			committed_branch_name: branch_name.to_string(),
			author: previous_build.author,
			commit_message: previous_build.git_commit_message,
		})
	} else if let Some(tag_name) =
		previous_build.git_ref.strip_prefix("refs/tags/")
	{
		EventType::Tag(Tag {
			repo_owner: repo.repo_owner,
			repo_name: repo.repo_name.clone(),
			commit_sha: previous_build.git_commit,
			tag_name: tag_name.to_string(),
			author: previous_build.author,
			commit_message: previous_build.git_commit_message,
		})
	} else if let Some(pull_number) = previous_build
		.git_ref
		.strip_prefix("refs/pull/")
		.and_then(|pr| pr.parse::<i64>().ok())
	{
		let github_client = octorust::Client::new(
			"patr",
			Credentials::Token(access_token.clone()),
		)
		.map_err(|err| {
			log::info!("error while octorust init: {err:#}");
			err
		})
		.ok()
		.ok_or_else(|| ErrorType::internal_error())?;

		let pr_details = github_client
			.pulls()
			.get(&repo.repo_owner, &repo.repo_name, pull_number)
			.await
			.map_err(|err| {
				log::info!("error while getting pull request details: {err:#}");
				err
			})
			.ok()
			.ok_or_else(|| ErrorType::internal_error())?;

		EventType::PullRequest(PullRequest {
			pr_repo_owner: pr_details
				.head
				.repo
				.as_ref()
				.map(|repo| repo.owner.login.clone())
				.unwrap_or_else(|| repo.repo_owner.clone()),
			pr_repo_name: pr_details
				.head
				.repo
				.map(|repo| repo.name)
				.unwrap_or_else(|| repo.repo_name.clone()),
			repo_owner: repo.repo_owner,
			repo_name: repo.repo_name.clone(),
			commit_sha: previous_build.git_commit,
			pr_number: pull_number.to_string(),
			author: previous_build.author,
			pr_title: previous_build.git_pr_title.unwrap_or_default(),
			to_be_committed_branch_name: pr_details.base.ref_,
		})
	} else {
		return Error::as_result()
			.ok_or_else(|| ErrorType::internal_error())?;
	};

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
			return;
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
				return;
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
			return;
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

	service::update_github_commit_status_for_build(
		&mut connection,
		&git_provider.workspace_id,
		&repo.id,
		build_num,
		CommitStatus::Running,
	)
	.await?;

	connection.commit().await?;

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

	Ok(RestartBuildResponse {
		build_num: build_num as u64,
	});
}

async fn start_build_for_branch(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: StartBuildPath {
			workspace_id,
			repo_id,
			branch_name,
		},
		query: (),
		body: (),
	}: DecodedRequest<StartBuildRequest>,
) -> Result<StartBuildResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {request_id} - Starting build for repo {repo_id} at branch {branch_name}");

	let repo = db::get_repo_details_using_github_uid_for_workspace(
		&mut connection,
		&workspace_id,
		&repo_id,
	)
	.await?
	.ok_or_else(|| ErrorType::internal_error())?;

	let git_provider = db::get_git_provider_details_by_id(
		&mut connection,
		&repo.git_provider_id,
	)
	.await?
	.ok_or_else(|| ErrorType::internal_error())?;

	let access_token = git_provider
		.password
		.ok_or_else(|| ErrorType::internal_error())?;

	let github_client =
		octorust::Client::new("patr", Credentials::Token(access_token.clone()))
			.map_err(|err| {
				log::info!("error while octorust init: {err:#}");
				err
			})
			.ok()
			.ok_or_else(|| ErrorType::internal_error())?;

	let github_branch = github_client
		.repos()
		.get_branch(&repo.repo_owner, &repo.repo_name, &branch_name)
		.await
		.map_err(|err| {
			log::info!("error while getting webhooks list: {err:#}");
			err
		})
		.ok()
		.ok_or_else(|| ErrorType::internal_error())?;

	let event_type = EventType::Commit(Commit {
		repo_owner: repo.repo_owner.clone(),
		repo_name: repo.repo_name.clone(),
		commit_sha: github_branch.commit.sha.clone(),
		committed_branch_name: branch_name,
		author: github_branch.commit.author.map(|author| author.name),
		commit_message: Some(github_branch.commit.commit.message),
	});

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
			return;
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
				return;
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
			return;
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
		&repo.id,
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

	Ok(StartBuildResponse {
		build_num: build_num as u64,
	});
}

async fn list_git_ref_for_repo(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: ListGitRefForRepoPath {
			workspace_id,
			repo_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<ListGitRefForRepoRequest>,
) -> Result<ListGitRefForRepoResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!(
		"request_id: {request_id} - Fetching all git ref for {repo_id}"
	);

	let repo = db::get_repo_details_using_github_uid_for_workspace(
		&mut connection,
		&workspace_id,
		&repo_id,
	)
	.await?
	.ok_or_else(|| ErrorType::internal_error())?;

	let git_provider = db::get_git_provider_details_by_id(
		&mut connection,
		&repo.git_provider_id,
	)
	.await?
	.ok_or_else(|| ErrorType::internal_error())?;
	let (_login_name, access_token) = git_provider
		.login_name
		.zip(git_provider.password)
		.ok_or_else(|| ErrorType::internal_error())?;

	let refs = service::list_git_ref_for_repo(
		&repo.repo_owner,
		&repo.repo_name,
		&access_token,
	)
	.await?;

	Ok(ListGitRefForRepoResponse { refs });
}

async fn get_patr_ci_file(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: GetPatrCiFilePath {
			workspace_id,
			repo_id,
			git_ref,
		},
		query: (),
		body: (),
	}: DecodedRequest<GetPatrCiFileRequest>,
) -> Result<GetPatrCiFileResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {request_id} - Fetching CI file for {repo_id} at ref {git_ref}");

	let repo = db::get_repo_details_using_github_uid_for_workspace(
		&mut connection,
		&workspace_id,
		&repo_id,
	)
	.await?
	.ok_or_else(|| ErrorType::internal_error())?;

	let git_provider = db::get_git_provider_details_by_id(
		&mut connection,
		&repo.git_provider_id,
	)
	.await?
	.ok_or_else(|| ErrorType::internal_error())?;
	let (_login_name, access_token) = git_provider
		.login_name
		.zip(git_provider.password)
		.ok_or_else(|| ErrorType::internal_error())?;

	let ci_file_content = service::fetch_ci_file_content_from_github_repo(
		&repo.repo_owner,
		&repo.repo_name,
		&git_ref,
		&access_token,
	)
	.await?;

	Ok(GetPatrCiFileResponse {
		file_content: Base64String::from(ci_file_content),
	});
}

async fn write_patr_ci_file(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: WritePatrCiFilePath {
			workspace_id,
			repo_id,
		},
		query: (),
		body:
			WritePatrCiFileRequest {
				commit_message,
				parent_commit_sha,
				branch_name,
				ci_file_content,
			},
	}: DecodedRequest<WritePatrCiFileRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();

	log::trace!(
		"request_id: {request_id} - Writing patr ci fiile to repo {repo_id}"
	);

	let repo = db::get_repo_details_using_github_uid_for_workspace(
		&mut connection,
		&workspace_id,
		&repo_id,
	)
	.await?
	.ok_or_else(|| ErrorType::internal_error())?;

	let git_provider = db::get_git_provider_details_by_id(
		&mut connection,
		&repo.git_provider_id,
	)
	.await?
	.ok_or_else(|| ErrorType::internal_error())?;

	let (_login_name, access_token) = git_provider
		.login_name
		.zip(git_provider.password)
		.ok_or_else(|| ErrorType::internal_error())?;

	service::write_ci_file_content_to_github_repo(
		&repo.repo_owner,
		&repo.repo_name,
		commit_message,
		parent_commit_sha,
		branch_name,
		ci_file_content,
		&access_token,
	)
	.await?;

	Ok(());
}

async fn sign_out(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: GithubSignOutPath { workspace_id },
		query: (),
		body: (),
	}: DecodedRequest<GithubSignOutRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {request_id} - Signout github from patr for workspace {workspace_id}");

	let git_provider = db::get_git_provider_details_for_workspace_using_domain(
		&mut connection,
		&workspace_id,
		"github.com",
	)
	.await?
	.ok_or_else(|| ErrorType::internal_error())?;
	let (_login_name, access_token) = git_provider
		.login_name
		.zip(git_provider.password)
		.ok_or_else(|| ErrorType::internal_error())?;

	db::remove_git_provider_credentials(&mut connection, &git_provider.id)
		.await?;

	let github_client =
		octorust::Client::new("patr", Credentials::Token(access_token))
			.map_err(|err| {
				log::info!("error while octorust init: {err:#}");
				err
			})
			.ok()
			.ok_or_else(|| ErrorType::internal_error())?;

	let repos =
		db::list_repos_for_git_provider(&mut connection, &git_provider.id)
			.await?
			.into_iter()
			.filter(|repo| repo.status == RepoStatus::Active)
			.collect::<Vec<_>>();

	for repo in repos {
		db::update_repo_status(
			&mut connection,
			&repo.git_provider_id,
			&repo.git_provider_repo_uid,
			RepoStatus::Inactive,
		)
		.await?;

		let webhooks = github_client
			.repos()
			.list_all_webhooks(&repo.repo_owner, &repo.repo_name)
			.await
			.map_err(|err| {
				log::info!("error while getting webhooks list: {err:#}");
				err
			})
			.ok()
			.ok_or_else(|| ErrorType::internal_error())?;

		let github_webhook_url =
			service::get_webhook_url_for_repo(&config.api_url, &repo.id);
		for webhook in webhooks {
			if webhook.config.url == github_webhook_url {
				github_client
					.repos()
					.delete_webhook(
						&repo.repo_owner,
						&repo.repo_name,
						webhook.id,
					)
					.await
					.map_err(|err| {
						log::info!("error while deleting webhook: {err:#}");
						err
					})
					.ok()
					.ok_or_else(|| ErrorType::internal_error())?;
			}
		}
	}

	Ok(());
}
