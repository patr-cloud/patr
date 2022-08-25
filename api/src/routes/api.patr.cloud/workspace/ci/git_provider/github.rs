use api_macros::closure_as_pinned_box;
use api_models::{
	models::workspace::ci::git_provider::{
		ActivateRepoRequest,
		ActivateRepoResponse,
		BuildLogs,
		BuildStatus,
		CancelBuildResponse,
		DeactivateRepoResponse,
		GetBuildInfoResponse,
		GetBuildListResponse,
		GetBuildLogResponse,
		GitProviderType,
		GithubAuthCallbackRequest,
		GithubAuthCallbackResponse,
		GithubAuthResponse,
		GithubSignOutResponse,
		ListReposResponse,
		RepoStatus,
		RepositoryDetails,
		RestartBuildResponse,
		SyncReposResponse,
	},
	utils::{DateTime, Uuid},
};
use chrono::Utc;
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use http::header::ACCEPT;
use octorust::{
	self,
	auth::Credentials,
	types::{ReposCreateWebhookRequest, ReposCreateWebhookRequestConfig},
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use redis::AsyncCommands;
use serde::Deserialize;

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::{ci::file_format::CiFlow, deployment::Logs, rbac::permissions},
	pin_fn,
	rabbitmq::{BuildId, BuildStepId},
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
	let mut app = create_eve_app(app);

	app.get(
		"/auth",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				// TODO: refactor permissions for ci
				permissions::workspace::ci::git_provider::CONNECT,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(connect_to_github)),
		],
	);

	app.post(
		"/auth-callback",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::ci::git_provider::CONNECT,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(github_oauth_callback)),
		],
	);

	app.post(
		"/repo/sync",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::ci::git_provider::repo::LIST,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(sync_repositories)),
		],
	);

	app.get(
		"/repo",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::ci::git_provider::repo::LIST,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(list_repositories)),
		],
	);

	app.post(
		"/repo/:repoId/activate",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::ci::git_provider::repo::ACTIVATE,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(activate_repo)),
		],
	);

	app.post(
		"/repo/:repoId/deactivate",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::ci::git_provider::repo::DEACTIVATE,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(deactivate_repo)),
		],
	);

	app.get(
		"/repo/:repoId/build",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::ci::git_provider::repo::build::VIEW,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(get_build_list)),
		],
	);

	app.get(
		"/repo/:repoId/build/:buildNum",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::ci::git_provider::repo::build::VIEW,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(get_build_info)),
		],
	);

	app.get(
		"/repo/:repoId/build/:buildNum/log/:step",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::ci::git_provider::repo::build::VIEW,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(get_build_logs)),
		],
	);

	app.post(
		"/repo/:repoId/build/:buildNum/cancel",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::ci::git_provider::repo::build::VIEW,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(cancel_build)),
		],
	);

	app.post(
		"/repo/:repoId/build/:buildNum/restart",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::ci::git_provider::repo::build::RESTART,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(restart_build)),
		],
	);

	app.delete(
		"/sign-out",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::ci::git_provider::DISCONNECT,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(sign_out)),
		],
	);

	app
}

async fn connect_to_github(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let client_id = context.get_state().config.github.client_id.to_owned();

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
	context
		.get_redis_connection()
		.set_ex(format!("githubAuthState:{state}"), value, ttl_in_secs)
		.await?;

	let oauth_url = format!("https://github.com/login/oauth/authorize?client_id={client_id}&scope={scope}&state={state}");
	context.success(GithubAuthResponse { oauth_url });
	Ok(context)
}

async fn github_oauth_callback(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let GithubAuthCallbackRequest { code, state, .. } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	// validate the state value
	let expected_value = workspace_id.as_str();
	let value_from_redis: Option<String> = context
		.get_redis_connection()
		.get(format!("githubAuthState:{state}"))
		.await?;
	if !value_from_redis
		.map(|value| value == expected_value)
		.unwrap_or(false)
	{
		Error::as_result()
			.status(400)
			.body(error!(INVALID_STATUS_VALUE).to_string())?
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
			(
				"client_id",
				context.get_state().config.github.client_id.clone(),
			),
			(
				"client_secret",
				context.get_state().config.github.client_secret.clone(),
			),
			("code", code),
		])
		.header(ACCEPT, "application/json")
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
			.status(500)?;

	let login_name = github_client
		.users()
		.get_authenticated_public_user()
		.await
		.map_err(|err| {
			log::info!("error while getting login name: {err:#}");
			err
		})
		.ok()
		.status(500)?
		.login;

	db::add_git_provider_to_workspace(
		context.get_database_connection(),
		&workspace_id,
		"github.com",
		GitProviderType::Github,
		Some(&login_name),
		Some(&access_token),
	)
	.await?;

	context.success(GithubAuthCallbackResponse {});
	Ok(context)
}

async fn sync_repositories(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	log::trace!("request_id: {request_id} - Syncing github repos for workspace {workspace_id}");

	let git_provider = db::get_git_provider_details_for_workspace_using_domain(
		context.get_database_connection(),
		&workspace_id,
		"github.com",
	)
	.await?
	.status(500)?;
	let (_login_name, access_token) = git_provider
		.login_name
		.zip(git_provider.password)
		.status(500)?;

	service::sync_github_repos(
		context.get_database_connection(),
		&git_provider.id,
		access_token,
		&request_id,
	)
	.await?;

	context.success(SyncReposResponse {});
	Ok(context)
}

async fn list_repositories(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	log::trace!("request_id: {request_id} - Listing github repos for workspace {workspace_id}");

	let git_provider = db::get_git_provider_details_for_workspace_using_domain(
		context.get_database_connection(),
		&workspace_id,
		"github.com",
	)
	.await?
	.status(500)?;

	let repos = db::list_repos_for_git_provider(
		context.get_database_connection(),
		&git_provider.id,
	)
	.await?
	.into_iter()
	.map(|repo| RepositoryDetails {
		id: repo.git_provider_repo_uid,
		name: repo.repo_name,
		repo_owner: repo.repo_owner,
		clone_url: repo.clone_url,
		status: repo.status,
		build_machine_type_id: repo.build_machine_type_id,
	})
	.collect();

	context.success(ListReposResponse { repos });
	Ok(context)
}

async fn activate_repo(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let repo_id = context.get_param(request_keys::REPO_ID).unwrap().clone();

	log::trace!("request_id: {request_id} - Activating CI for repo {repo_id}");

	let repo = db::get_repo_details_using_github_uid_for_workspace(
		context.get_database_connection(),
		&workspace_id,
		&repo_id,
	)
	.await?
	.status(500)?;

	let git_provider = db::get_connected_git_provider_details_by_id(
		context.get_database_connection(),
		&repo.git_provider_id,
	)
	.await?
	.status(500)?;
	let (_login_name, access_token) = git_provider
		.login_name
		.zip(git_provider.password)
		.status(500)?;

	let ActivateRepoRequest {
		workspace_id: _,
		repo_id: _,
		build_machine_type_id,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let github_client =
		octorust::Client::new("patr", Credentials::Token(access_token))
			.map_err(|err| {
				log::info!("error while octorust init: {err:#}");
				err
			})
			.ok()
			.status(500)?;

	let webhook_secret = db::activate_ci_for_repo(
		context.get_database_connection(),
		&repo.id,
		&build_machine_type_id,
	)
	.await?;

	let github_webhook_url =
		context.get_state().config.callback_domain_url.clone() + "/webhook/ci";

	// TODO:
	// - either store the returned webhook id and then during deactivation use
	//   that hook id
	// - else create a custome webhook url for each repo based on repo_id and
	//   then use it (revaluate)
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
					url: github_webhook_url,
				}),
				events: vec!["push".to_string()],
				name: "web".to_string(),
			},
		)
		.await
		.map_err(|err| {
			log::info!("error while configuring webhooks: {err:#}");
			err
		})
		.ok()
		.status(500)?;

	context.success(ActivateRepoResponse {});
	Ok(context)
}

async fn deactivate_repo(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let repo_id = context.get_param(request_keys::REPO_ID).unwrap().clone();

	log::trace!(
		"request_id: {request_id} - Deactivating CI for repo {repo_id}"
	);

	let repo = db::get_repo_details_using_github_uid_for_workspace(
		context.get_database_connection(),
		&workspace_id,
		&repo_id,
	)
	.await?
	.status(500)?;

	let git_provider = db::get_connected_git_provider_details_by_id(
		context.get_database_connection(),
		&repo.git_provider_id,
	)
	.await?
	.status(500)?;
	let (_login_name, access_token) = git_provider
		.login_name
		.zip(git_provider.password)
		.status(500)?;

	let github_client =
		octorust::Client::new("patr", Credentials::Token(access_token))
			.map_err(|err| {
				log::info!("error while octorust init: {err:#}");
				err
			})
			.ok()
			.status(500)?;

	db::update_repo_status(
		context.get_database_connection(),
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
		.status(500)?;

	let github_webhook_url =
		context.get_state().config.callback_domain_url.clone() + "/webhook/ci";

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
				.status(500)?;
		}
	}

	context.success(DeactivateRepoResponse {});
	Ok(context)
}

async fn get_build_list(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let repo_id = context.get_param(request_keys::REPO_ID).unwrap().clone();

	log::trace!(
		"request_id: {request_id} - Getting build list for repo {repo_id}"
	);

	let repo = db::get_repo_details_using_github_uid_for_workspace(
		context.get_database_connection(),
		&workspace_id,
		&repo_id,
	)
	.await?
	.status(500)?;

	let builds = db::list_build_details_for_repo(
		context.get_database_connection(),
		&repo.id,
	)
	.await?;

	context.success(GetBuildListResponse { builds });
	Ok(context)
}

async fn get_build_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let repo_id = context.get_param(request_keys::REPO_ID).unwrap().clone();
	let build_num = context
		.get_param(request_keys::BUILD_NUM)
		.unwrap()
		.parse::<i64>()?;

	log::trace!("request_id: {request_id} - Getting build info for repo {repo_id} - {build_num}");

	let repo = db::get_repo_details_using_github_uid_for_workspace(
		context.get_database_connection(),
		&workspace_id,
		&repo_id,
	)
	.await?
	.status(500)?;

	let build_info = db::get_build_details_for_build(
		context.get_database_connection(),
		&repo.id,
		build_num as i64,
	)
	.await?
	.status(400)
	.body("build not found")?;

	context.success(GetBuildInfoResponse { build_info });
	Ok(context)
}

async fn get_build_logs(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let repo_id = context.get_param(request_keys::REPO_ID).unwrap().clone();
	let build_num = context
		.get_param(request_keys::BUILD_NUM)
		.unwrap()
		.parse::<i64>()?;
	let step = context
		.get_param(request_keys::STEP)
		.unwrap()
		.parse::<i32>()?;

	log::trace!("request_id: {request_id} - Getting build logs for repo {repo_id} - {build_num} - {step}");

	let repo = db::get_repo_details_using_github_uid_for_workspace(
		context.get_database_connection(),
		&workspace_id,
		&repo_id,
	)
	.await?
	.status(500)?;

	let build_created_time = db::get_build_created_time(
		context.get_database_connection(),
		&repo.id,
		build_num as i64,
	)
	.await?
	.status(500)
	.body(error!(SERVER_ERROR).to_string())?;

	let build_step_id = BuildStepId {
		build_id: BuildId {
			repo_workspace_id: workspace_id,
			repo_id: repo.id,
			build_num,
		},
		step_id: step,
	};

	let loki = context.get_state().config.loki.clone();
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

	context.success(GetBuildLogResponse { logs });
	Ok(context)
}

async fn cancel_build(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let repo_id = context.get_param(request_keys::REPO_ID).unwrap().clone();
	let build_num = context
		.get_param(request_keys::BUILD_NUM)
		.unwrap()
		.parse::<i64>()?;

	log::trace!("request_id: {request_id} - Stopping build for repo {repo_id} - {build_num}");

	let repo = db::get_repo_details_using_github_uid_for_workspace(
		context.get_database_connection(),
		&workspace_id,
		&repo_id,
	)
	.await?
	.status(500)?;

	let build = db::get_build_details_for_build(
		context.get_database_connection(),
		&repo.id,
		build_num,
	)
	.await?
	.status(400)
	.body("Build does not exists")?;

	if build.status == BuildStatus::Running {
		service::queue_cancel_ci_build_pipeline(
			BuildId {
				repo_workspace_id: workspace_id,
				repo_id: repo.id,
				build_num,
			},
			&context.get_state().config,
			&request_id,
		)
		.await?;
	}

	context.success(CancelBuildResponse {});
	Ok(context)
}

async fn restart_build(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let repo_id = context.get_param(request_keys::REPO_ID).unwrap().clone();
	let build_num = context
		.get_param(request_keys::BUILD_NUM)
		.unwrap()
		.parse::<i64>()?;

	log::trace!("request_id: {request_id} - Restarting build for repo {repo_id} - {build_num}");

	let repo = db::get_repo_details_using_github_uid_for_workspace(
		context.get_database_connection(),
		&workspace_id,
		&repo_id,
	)
	.await?
	.status(500)?;

	let git_provider = db::get_connected_git_provider_details_by_id(
		context.get_database_connection(),
		&repo.git_provider_id,
	)
	.await?
	.status(500)?;
	let (login_name, access_token) = git_provider
		.login_name
		.zip(git_provider.password)
		.status(500)?;

	let previous_build = db::get_build_details_for_build(
		context.get_database_connection(),
		&repo.id,
		build_num as i64,
	)
	.await?
	.status(500)?;

	let config = context.get_state().config.clone();
	let git_commit = previous_build.git_commit.as_ref();

	let ci_file_content = service::fetch_ci_file_content_from_github_repo(
		&repo.repo_owner,
		&repo.repo_name,
		&access_token,
		git_commit,
	)
	.await?;

	let build_num = db::generate_new_build_for_repo(
		context.get_database_connection(),
		&repo.id,
		&previous_build.git_ref,
		&previous_build.git_commit,
		BuildStatus::Running,
		&DateTime::from(Utc::now()),
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

	let branch_name = previous_build.git_ref.strip_prefix("refs/heads/");

	let CiFlow::Pipeline(pipeline) = ci_flow;
	let works = match service::evaluate_work_steps_for_ci(
		pipeline.steps,
		branch_name,
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

	service::add_build_steps_in_k8s(
		context.get_database_connection(),
		&config,
		&repo.id,
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
		&repo.clone_url,
		&repo.repo_name,
		git_commit,
		&request_id,
	)
	.await?;

	context.success(RestartBuildResponse {
		build_num: build_num as u64,
	});
	Ok(context)
}

async fn sign_out(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	log::trace!("request_id: {request_id} - Signout github from patr for workspace {workspace_id}");

	let git_provider = db::get_git_provider_details_for_workspace_using_domain(
		context.get_database_connection(),
		&workspace_id,
		"github.com",
	)
	.await?
	.status(500)?;
	let (_login_name, access_token) = git_provider
		.login_name
		.zip(git_provider.password)
		.status(500)?;

	db::remove_git_provider_credentials(
		context.get_database_connection(),
		&git_provider.id,
	)
	.await?;

	let github_client =
		octorust::Client::new("patr", Credentials::Token(access_token))
			.map_err(|err| {
				log::info!("error while octorust init: {err:#}");
				err
			})
			.ok()
			.status(500)?;

	let repos = db::list_repos_for_git_provider(
		context.get_database_connection(),
		&git_provider.id,
	)
	.await?
	.into_iter()
	.filter(|repo| repo.status == RepoStatus::Active)
	.collect::<Vec<_>>();

	let github_webhook_url =
		context.get_state().config.callback_domain_url.clone() + "/webhook/ci";
	for repo in repos {
		db::update_repo_status(
			context.get_database_connection(),
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
			.status(500)?;

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
					.status(500)?;
			}
		}
	}

	// TODO: Now show a pop-up / github redirect in UI for deleting patr in
	// the user's authorized github oauth apps

	context.success(GithubSignOutResponse {});
	Ok(context)
}
