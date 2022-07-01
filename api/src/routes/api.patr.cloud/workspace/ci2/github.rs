use api_macros::closure_as_pinned_box;
use api_models::{
	models::workspace::ci2::github::{
		ActivateGithubRepoResponse,
		DeactivateGithubRepoResponse,
		GithubAuthCallbackRequest,
		GithubAuthCallbackResponse,
		GithubAuthResponse,
		GithubListReposResponse,
		GithubRepository,
	},
	utils::Uuid,
};
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
		WebhookConfigInsecureSslOneOf,
	},
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use redis::AsyncCommands;
use serde::Deserialize;

use crate::{
	app::{create_eve_app, App},
	db::{self, activate_ci_for_repo},
	error,
	models::rbac::permissions,
	pin_fn,
	utils::{
		constants::request_keys,
		Error,
		ErrorData,
		EveContext,
		EveMiddleware,
	},
};

/// # Description
/// This function is used to create a sub app for every endpoint listed. It
/// creates an eve app which binds the endpoint with functions.
///
/// # Arguments
/// * `app` - an object of type [`App`] which contains all the configuration of
///   api including the
/// database connections.
///
/// # Returns
/// this function returns `EveApp<EveContext, EveMiddleware, App, ErrorData>`
/// containing context, middleware, object of [`App`] and Error
///
/// [`App`]: App
pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut app = create_eve_app(app);

	app.get(
		"/auth",
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
	/*
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
		"/repo/:repoOwner/:repoName/build/:buildNum/log/:stage/:step",
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
	*/
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
	let scope = "repo"; // TODO

	// https://docs.github.com/en/developers/apps/building-oauth-apps/authorizing-oauth-apps#2-users-are-redirected-back-to-your-site-by-github
	let state = thread_rng()
		.sample_iter(&Alphanumeric)
		.take(8)
		.map(char::from)
		.collect::<String>();

	// temporary code from github will expire within 10 mins so we are using
	// extra 5 mins as buffer time for now
	let ttl_in_secs = 15 * 60; // 15 mins
	let value = workspace_id.as_str(); // TODO: user/workspace ?
	context
		.get_redis_connection()
		.set_ex(format!("githubAuthState:{state}"), value, ttl_in_secs)
		.await?;

	let oauth_url = format!("https://github.com/login/oauth/authorize?client_id={client_id}&scope={scope}&state={state}"); // TODO: state
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
	let expected_value = workspace_id.as_str(); // TODO: user/workspace ?
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

	// TODO: update our local db and then fetch from db -> or we can use a
	// crawler (highly complex) TODO: create a scheduler to check whether
	// webhook has been removed manually TODO: test with org accounts
	// TODO: how to handle whether the repo ci active or not during list repos
	let repos = github_client
		.repos()
		.list_all_for_authenticated_user(
			ReposListVisibility::All,
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
		.map(|repo| GithubRepository {
			name: repo.name,
			description: repo.description,
			full_name: repo.full_name,
			git_url: repo.git_url,
			owner: repo.owner.map(|user| user.name),
			organization: repo.organization.map(|org| org.name),
		})
		.collect();

	context.success(GithubListReposResponse { repos });
	Ok(context)
}

const GITHUB_WEBHOOK_URL: &str =
	"https://01d7-106-200-254-12.in.ngrok.io/webhook/ci/push-event"; // TODO

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

	let repo = if let Some(repo) = db::get_repo_for_workspace_and_url(
		context.get_database_connection(),
		&workspace_id,
		&repo.git_url,
	)
	.await?
	{
		repo
	} else {
		db::create_ci_repo(
			context.get_database_connection(),
			&workspace_id,
			&repo.git_url,
		)
		.await?
	};

	activate_ci_for_repo(context.get_database_connection(), &repo.id).await?;

	// TODO: better to store hook id in db so that we can modify that alone
	let _configured_webhook = github_client
		.repos()
		.create_webhook(
			&repo_owner,
			&repo_name,
			&ReposCreateWebhookRequest {
				active: Some(true),
				config: Some(ReposCreateWebhookRequestConfig {
					content_type: "json".to_string(),
					digest: "".to_string(),
					insecure_ssl: Some(WebhookConfigInsecureSslOneOf::String(
						"1".to_string(), // TODO: switch to ssl
					)),
					secret: repo.webhook_secret,
					token: "".to_string(),
					url: GITHUB_WEBHOOK_URL.to_string(),
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

	let repo = db::get_repo_for_workspace_and_url(
		context.get_database_connection(),
		&workspace_id,
		&repo.git_url,
	)
	.await?
	.status(400)
	.body("repo not found")?;

	db::deactivate_ci_for_repo(context.get_database_connection(), &repo.id)
		.await?;

	// store the particular registed github webhook id and delete that alone
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

	for webhook in all_webhooks {
		if webhook.config.url == GITHUB_WEBHOOK_URL {
			let _ = github_client
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

/*
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

	let (_, drone_token) = db::get_drone_username_and_token_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.status(404)
	.body(error!(USER_NOT_FOUND).to_string())?;

	let builds = reqwest::Client::new()
		.get(format!(
			"{}/api/repos/{}/{}/builds",
			context.get_state().config.drone.url,
			repo_owner,
			repo_name
		))
		.bearer_auth(drone_token)
		.send()
		.await?
		.error_for_status()?
		.json()
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

	let (_, drone_token) = db::get_drone_username_and_token_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.status(404)
	.body(error!(USER_NOT_FOUND).to_string())?;

	let build_info = reqwest::Client::new()
		.get(format!(
			"{}/api/repos/{}/{}/builds/{}",
			context.get_state().config.drone.url,
			repo_owner,
			repo_name,
			build_num
		))
		.bearer_auth(drone_token)
		.send()
		.await?
		.error_for_status()?
		.json()
		.await?;

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
		.parse::<u64>()?;
	let stage = context
		.get_param(request_keys::STAGE)
		.unwrap()
		.parse::<u64>()?;
	let step = context
		.get_param(request_keys::STEP)
		.unwrap()
		.parse::<u64>()?;

	let (_, drone_token) = db::get_drone_username_and_token_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.status(404)
	.body(error!(USER_NOT_FOUND).to_string())?;

	let response = reqwest::Client::new()
		.get(format!(
			"{}/api/repos/{}/{}/builds/{}/logs/{}/{}",
			context.get_state().config.drone.url,
			repo_owner,
			repo_name,
			build_num,
			stage,
			step
		))
		.bearer_auth(drone_token)
		.send()
		.await?;

	if response.status().as_u16() == 404 {
		context.status(404).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::NOT_FOUND,
			request_keys::MESSAGE: error::message::NOT_FOUND,
		}));
		return Ok(context);
	}

	let logs = response.error_for_status()?.json().await?;

	context.success(GetBuildLogResponse { logs });
	Ok(context)
}

async fn restart_build(
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

	let (_, drone_token) = db::get_drone_username_and_token_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.status(404)
	.body(error!(USER_NOT_FOUND).to_string())?;

	let build_num = reqwest::Client::new()
		.post(format!(
			"{}/api/repos/{}/{}/builds/{}",
			context.get_state().config.drone.url,
			repo_owner,
			repo_name,
			build_num
		))
		.bearer_auth(drone_token)
		.send()
		.await?
		.error_for_status()?
		.json::<BuildDetails>()
		.await?
		.number;

	context.success(RestartBuildResponse { build_num });
	Ok(context)
}

async fn sign_out(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let (_, drone_token) = db::get_drone_username_and_token_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.status(404)
	.body(error!(USER_NOT_FOUND).to_string())?;

	db::remove_drone_username_and_token_from_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?;

	let client = reqwest::Client::new();
	let repos = client
		.get(format!(
			"{}/api/user/repos",
			context.get_state().config.drone.url
		))
		.bearer_auth(&drone_token)
		.send()
		.await?
		.error_for_status()?
		.json::<Vec<GithubRepository>>()
		.await?
		.into_iter()
		.filter(|repo| repo.active)
		.collect::<Vec<_>>();

	for repo in repos {
		client
			.delete(format!(
				"{}/api/repos/{}/{}",
				context.get_state().config.drone.url,
				repo.namespace,
				repo.name
			))
			.bearer_auth(&drone_token)
			.send()
			.await?
			.error_for_status()?;
	}

	context.success(GithubSignOutResponse {});
	Ok(context)
}
*/
