use std::collections::HashMap;

use api_macros::closure_as_pinned_box;
use api_models::{
	models::workspace::ci2::github::{
		ActivateGithubRepoRequest,
		ActivateGithubRepoResponse,
		BuildLogs,
		BuildStatus,
		DeactivateGithubRepoResponse,
		GetBuildInfoResponse,
		GetBuildListResponse,
		GetBuildLogResponse,
		GithubAuthCallbackRequest,
		GithubAuthCallbackResponse,
		GithubAuthResponse,
		GithubListReposResponse,
		GithubRepository,
		GithubSignOutResponse,
		RestartBuildResponse,
		StopBuildResponse,
	},
	utils::Uuid,
};
use chrono::Utc;
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use http::header::ACCEPT;
use octorust::{
	self,
	auth::Credentials,
	types::{
		Order,
		ReposCreateWebhookRequest,
		ReposCreateWebhookRequestConfig,
		ReposListOrgSort,
		ReposListVisibility,
	},
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use redis::AsyncCommands;
use serde::Deserialize;

use crate::{
	app::{create_eve_app, App},
	db::{self, get_all_repos_for_workspace},
	error,
	models::{deployment::Logs, rbac::permissions},
	pin_fn,
	rabbitmq::{BuildId, BuildStepId},
	service::{self, ParseStatus, Netrc},
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
				permissions::workspace::ci::github::CONNECT,
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
				permissions::workspace::ci::github::CONNECT,
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

	app.get(
		"/repo",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::ci::github::VIEW_BUILDS,
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
		"/repo/:repoOwner/:repoName/activate",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::ci::github::ACTIVATE,
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
		"/repo/:repoOwner/:repoName/deactivate",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::ci::github::DEACTIVATE,
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
		"/repo/:repoOwner/:repoName/build",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::ci::github::VIEW_BUILDS,
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
		"/repo/:repoOwner/:repoName/build/:buildNum",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::ci::github::VIEW_BUILDS,
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
		"/repo/:repoOwner/:repoName/build/:buildNum/log/:step",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::ci::github::VIEW_BUILDS,
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
		"/repo/:repoOwner/:repoName/build/:buildNum/stop",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::ci::github::VIEW_BUILDS,
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
			EveMiddleware::CustomFunction(pin_fn!(stop_build)),
		],
	);

	app.post(
		"/repo/:repoOwner/:repoName/build/:buildNum/restart",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::ci::github::RESTART_BUILDS,
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
				permissions::workspace::ci::github::DISCONNECT,
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
		Error::as_result().status(400).body("invalid state value")?
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

	let login = github_client
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

	db::set_drone_username_and_token_for_workspace(
		context.get_database_connection(),
		&workspace_id,
		&login,
		&access_token,
	)
	.await?;

	context.success(GithubAuthCallbackResponse {});
	Ok(context)
}

async fn list_repositories(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let ci_status_for_repo = db::get_all_repos_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.into_iter()
	.map(|repo| (repo.git_url, (repo.active, repo.build_machine_type_id)))
	.collect::<HashMap<_, _>>();

	let (_, access_token) = db::get_drone_username_and_token_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.status(404)
	.body(error!(USER_NOT_FOUND).to_string())?;

	let github_client =
		octorust::Client::new("patr", Credentials::Token(access_token.clone()))
			.map_err(|err| {
				log::info!("error while octorust init: {err:#}");
				err
			})
			.ok()
			.status(500)?;

	// TODO: create a scheduler to check whether webhook has been removed
	let repos = github_client
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
		.map(|repo| {
			let (repo_owner, repo_name) =
				repo.full_name.rsplit_once('/').unwrap(); // TODO

			let (is_ci_active, build_machine_type_id) = ci_status_for_repo
				.get(&repo.clone_url)
				.map_or((false, None), |(status, machine_type)| {
					(*status, Some(machine_type.to_owned()))
				});
			GithubRepository {
				name: repo_name.to_string(),
				description: repo.description,
				is_ci_active,
				git_url: repo.clone_url,
				repo_owner: repo_owner.to_string(),
				organization: repo.organization.map(|org| org.name),
				build_machine_type_id,
			}
		})
		.collect();

	context.success(GithubListReposResponse { repos });
	Ok(context)
}

async fn activate_repo(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let repo_owner =
		context.get_param(request_keys::REPO_OWNER).unwrap().clone();
	let repo_name = context.get_param(request_keys::REPO_NAME).unwrap().clone();

	let ActivateGithubRepoRequest {
		workspace_id: _,
		repo_owner: _,
		repo_name: _,
		build_machine_type_id,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let (_, access_token) = db::get_drone_username_and_token_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.status(404)
	.body(error!(USER_NOT_FOUND).to_string())?;

	let github_client =
		octorust::Client::new("patr", Credentials::Token(access_token))
			.map_err(|err| {
				log::info!("error while octorust init: {err:#}");
				err
			})
			.ok()
			.status(500)?;

	let repo = github_client
		.repos()
		.get(&repo_owner, &repo_name)
		.await
		.map_err(|err| {
			log::info!("error while getting repo info: {err:#}");
			err
		})
		.ok()
		.status(500)?;

	let (repo_owner, repo_name) =
		repo.full_name.rsplit_once('/').status(500)?;

	let repo = if let Some(repo) = db::get_repo_for_workspace_and_url(
		context.get_database_connection(),
		&workspace_id,
		&repo.clone_url,
	)
	.await?
	{
		db::update_build_machine_type_for_repo(
			context.get_database_connection(),
			&repo.id,
			&build_machine_type_id,
		)
		.await?;
		repo
	} else {
		db::create_ci_repo(
			context.get_database_connection(),
			&workspace_id,
			repo_owner,
			repo_name,
			&repo.clone_url,
			&build_machine_type_id,
		)
		.await?
	};

	db::activate_ci_for_repo(context.get_database_connection(), &repo.id)
		.await?;

	let github_webhook_url =
		context.get_state().config.callback_domain_url.clone() + "/webhook/ci";

	// TODO: its better to store hook id in db so that
	// we can update that alone during activate and deactivate actions
	let _configured_webhook = github_client
		.repos()
		.create_webhook(
			repo_owner,
			repo_name,
			&ReposCreateWebhookRequest {
				active: Some(true),
				config: Some(ReposCreateWebhookRequestConfig {
					content_type: "json".to_string(),
					digest: "".to_string(),
					insecure_ssl: None,
					secret: repo.webhook_secret,
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

	context.success(ActivateGithubRepoResponse {});
	Ok(context)
}

async fn deactivate_repo(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let repo_owner =
		context.get_param(request_keys::REPO_OWNER).unwrap().clone();
	let repo_name = context.get_param(request_keys::REPO_NAME).unwrap().clone();

	let repo = db::get_repo_for_workspace(
		context.get_database_connection(),
		&workspace_id,
		&repo_owner,
		&repo_name,
	)
	.await?
	.status(500)?;

	let (_, access_token) = db::get_drone_username_and_token_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.status(404)
	.body(error!(USER_NOT_FOUND).to_string())?;

	let github_client =
		octorust::Client::new("patr", Credentials::Token(access_token))
			.map_err(|err| {
				log::info!("error while octorust init: {err:#}");
				err
			})
			.ok()
			.status(500)?;

	db::deactivate_ci_for_repo(context.get_database_connection(), &repo.id)
		.await?;

	// TODO: store the particular registed github webhook id and delete that
	// alone
	let all_webhooks = github_client
		.repos()
		.list_all_webhooks(&repo_owner, &repo_name)
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
				.delete_webhook(&repo_owner, &repo_name, webhook.id)
				.await
				.map_err(|err| {
					log::info!("error while deleting webhook: {err:#}");
					err
				})
				.ok()
				.status(500)?;
		}
	}

	context.success(DeactivateGithubRepoResponse {});
	Ok(context)
}

async fn get_build_list(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let repo_owner =
		context.get_param(request_keys::REPO_OWNER).unwrap().clone();
	let repo_name = context.get_param(request_keys::REPO_NAME).unwrap().clone();

	let repo = db::get_repo_for_workspace(
		context.get_database_connection(),
		&workspace_id,
		&repo_owner,
		&repo_name,
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
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let repo_owner =
		context.get_param(request_keys::REPO_OWNER).unwrap().clone();
	let repo_name = context.get_param(request_keys::REPO_NAME).unwrap().clone();
	let build_num = context
		.get_param(request_keys::BUILD_NUM)
		.unwrap()
		.parse::<u64>()?;

	let repo = db::get_repo_for_workspace(
		context.get_database_connection(),
		&workspace_id,
		&repo_owner,
		&repo_name,
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
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let repo_owner =
		context.get_param(request_keys::REPO_OWNER).unwrap().clone();
	let repo_name = context.get_param(request_keys::REPO_NAME).unwrap().clone();
	let build_num = context
		.get_param(request_keys::BUILD_NUM)
		.unwrap()
		.parse::<i64>()?;
	let step = context
		.get_param(request_keys::STEP)
		.unwrap()
		.parse::<i32>()?;

	let repo = db::get_repo_for_workspace(
		context.get_database_connection(),
		&workspace_id,
		&repo_owner,
		&repo_name,
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

async fn stop_build(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let repo_owner =
		context.get_param(request_keys::REPO_OWNER).unwrap().clone();
	let repo_name = context.get_param(request_keys::REPO_NAME).unwrap().clone();
	let build_num = context
		.get_param(request_keys::BUILD_NUM)
		.unwrap()
		.parse::<i64>()?;

	let repo = db::get_repo_for_workspace(
		context.get_database_connection(),
		&workspace_id,
		&repo_owner,
		&repo_name,
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
		service::queue_stop_ci_build_pipeline(
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

	context.success(StopBuildResponse {});
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

	let repo_owner =
		context.get_param(request_keys::REPO_OWNER).unwrap().clone();
	let repo_name = context.get_param(request_keys::REPO_NAME).unwrap().clone();
	let build_num = context
		.get_param(request_keys::BUILD_NUM)
		.unwrap()
		.parse::<u64>()?;

	let repo = db::get_repo_for_workspace(
		context.get_database_connection(),
		&workspace_id,
		&repo_owner,
		&repo_name,
	)
	.await?
	.status(500)?;

	let previous_build = db::get_build_details_for_build(
		context.get_database_connection(),
		&repo.id,
		build_num as i64,
	)
	.await?
	.status(400)?;

	let access_token = db::get_access_token_for_repo(
		context.get_database_connection(),
		&repo.id,
	)
	.await?
	.status(500)
	.body("internal server error")?;

	let config = context.get_state().config.clone();
	let git_commit = previous_build.git_commit.as_ref();

	let ci_file_content = service::fetch_ci_file_content_from_github_repo(
		&repo_owner,
		&repo_name,
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
		&Utc::now(),
	)
	.await?;

	let ci_flow = match service::parse_ci_file_content(
		context.get_database_connection(),
		&repo.workspace_id,
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

	service::add_build_steps_in_db(
		context.get_database_connection(),
		&repo.id,
		build_num,
		&ci_flow,
		&request_id
	)
	.await?;

	context.commit_database_transaction().await?;

	service::add_build_steps_in_k8s(
		context.get_database_connection(),
		&config,
		&repo.id,
		&BuildId {
			repo_workspace_id: repo.workspace_id,
			repo_id: repo.id.clone(),
			build_num,
		},
		ci_flow,
		Some(Netrc {
			machine: "github.com".to_owned(),
			login: "oauth".to_owned(),
			password: access_token,
		}),
		&repo.git_url,
		&repo_name,
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
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let (_, access_token) = db::get_drone_username_and_token_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.status(404)
	.body(error!(USER_NOT_FOUND).to_string())?;

	let github_webhook_url =
		context.get_state().config.callback_domain_url.clone() + "/webhook/ci";

	db::remove_drone_username_and_token_from_workspace(
		context.get_database_connection(),
		&workspace_id,
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

	let repos = get_all_repos_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?;

	for repo in repos {
		db::deactivate_ci_for_repo(context.get_database_connection(), &repo.id)
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

	// TODO: now show in client to delete patr in github oauth apps

	context.success(GithubSignOutResponse {});
	Ok(context)
}
