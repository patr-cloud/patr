use api_macros::closure_as_pinned_box;
use api_models::{
	models::{
		ci::file_format::CiFlow,
		workspace::ci::git_provider::{
			ActivateRepoRequest,
			ActivateRepoResponse,
			AddRepoToWorkspaceResponse,
			BuildLogs,
			BuildStatus,
			CancelBuildResponse,
			DeactivateRepoResponse,
			DeleteRepoResponse,
			GetBuildInfoResponse,
			GetBuildListResponse,
			GetBuildLogResponse,
			GetPatrCiFileResponse,
			GetRepoInfoResponse,
			GitProviderType,
			GithubAppInstallRequest,
			GithubAppInstallResponse,
			GithubAuthCallbackRequest,
			GithubAuthCallbackResponse,
			GithubAuthResponse,
			GithubSignOutResponse,
			ListGitRefForRepoResponse,
			ListUserReposResponse,
			ListWorkspaceReposResponse,
			RepositoryDetails,
			RestartBuildResponse,
			SyncReposResponse,
			WorkspaceRepositoryDetails,
			WritePatrCiFileRequest,
			WritePatrCiFileResponse,
		},
	},
	utils::{Base64String, Uuid},
};
use chrono::Utc;
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use octorust::{self, auth::Credentials};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use redis::AsyncCommands;
use serde::Deserialize;

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::{
		ci::{github::CommitStatus, Commit, EventType, PullRequest, Tag},
		deployment::Logs,
		rbac::{self, permissions},
	},
	pin_fn,
	rabbitmq::{BuildId, BuildStepId},
	service::{self, ParseStatus},
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
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: false,
				permission: permissions::workspace::ci::git_provider::CONNECT,
				resource: closure_as_pinned_box!(|mut context| {
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
			},
			EveMiddleware::CustomFunction(pin_fn!(connect_to_github)),
		],
	);

	app.post(
		"/auth-callback",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: false,
				permission: permissions::workspace::ci::git_provider::CONNECT,
				resource: closure_as_pinned_box!(|mut context| {
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
			},
			EveMiddleware::CustomFunction(pin_fn!(github_oauth_callback)),
		],
	);

	app.post(
		"/auth/done",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: false,
				permission: permissions::workspace::ci::git_provider::CONNECT,
				resource: closure_as_pinned_box!(|mut context| {
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
			},
			EveMiddleware::CustomFunction(pin_fn!(github_app_install)),
		],
	);

	app.post(
		"/repo/sync",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::ci::git_provider::repo::LIST,
				resource: closure_as_pinned_box!(|mut context| {
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
			},
			EveMiddleware::CustomFunction(pin_fn!(sync_repositories)),
		],
	);

	app.get(
		"/repo",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::ci::git_provider::repo::LIST,
				resource: closure_as_pinned_box!(|mut context| {
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
			},
			EveMiddleware::CustomFunction(pin_fn!(list_user_repositories)),
		],
	);

	app.get(
		"/repo/workspace",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::ci::git_provider::repo::LIST,
				resource: closure_as_pinned_box!(|mut context| {
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
			},
			EveMiddleware::CustomFunction(pin_fn!(list_workspace_repositories)),
		],
	);

	app.get(
		"/repo/:repoId",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::ci::git_provider::repo::ACTIVATE,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id = Uuid::parse_str(
						context.get_param(request_keys::WORKSPACE_ID).unwrap(),
					)
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
			},
			EveMiddleware::CustomFunction(pin_fn!(get_repo_info)),
		],
	);

	app.post(
		"/repo/:repoId/activate",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::ci::git_provider::repo::ACTIVATE,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id = Uuid::parse_str(
						context.get_param(request_keys::WORKSPACE_ID).unwrap(),
					)
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
			},
			EveMiddleware::CustomFunction(pin_fn!(activate_repo)),
		],
	);

	app.post(
		"/repo/:repoId/add-to-workspace",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::ci::git_provider::repo::ACTIVATE,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id = Uuid::parse_str(
						context.get_param(request_keys::WORKSPACE_ID).unwrap(),
					)
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
			},
			EveMiddleware::CustomFunction(pin_fn!(add_repo_to_workspace)),
		],
	);

	app.post(
		"/repo/:repoId/deactivate",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::ci::git_provider::repo::DEACTIVATE,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id = Uuid::parse_str(
						context.get_param(request_keys::WORKSPACE_ID).unwrap(),
					)
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
			},
			EveMiddleware::CustomFunction(pin_fn!(deactivate_repo)),
		],
	);

	app.delete(
		"/repo/:repoId/workspace-repo",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::ci::git_provider::repo::DEACTIVATE,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id = Uuid::parse_str(
						context.get_param(request_keys::WORKSPACE_ID).unwrap(),
					)
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
			},
			EveMiddleware::CustomFunction(pin_fn!(delete_workspace_repo)),
		],
	);

	app.get(
		"/repo/:repoId/build",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::ci::git_provider::repo::build::LIST,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id = Uuid::parse_str(
						context.get_param(request_keys::WORKSPACE_ID).unwrap(),
					)
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
			},
			EveMiddleware::CustomFunction(pin_fn!(get_build_list)),
		],
	);

	app.get(
		"/repo/:repoId/build/:buildNum",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::ci::git_provider::repo::build::INFO,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id = Uuid::parse_str(
						context.get_param(request_keys::WORKSPACE_ID).unwrap(),
					)
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
			},
			EveMiddleware::CustomFunction(pin_fn!(get_build_info)),
		],
	);

	app.get(
		"/repo/:repoId/build/:buildNum/log/:step",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::ci::git_provider::repo::build::INFO,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id = Uuid::parse_str(
						context.get_param(request_keys::WORKSPACE_ID).unwrap(),
					)
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
			},
			EveMiddleware::CustomFunction(pin_fn!(get_build_logs)),
		],
	);

	app.post(
		"/repo/:repoId/build/:buildNum/cancel",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::ci::git_provider::repo::build::CANCEL,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id = Uuid::parse_str(
						context.get_param(request_keys::WORKSPACE_ID).unwrap(),
					)
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
			},
			EveMiddleware::CustomFunction(pin_fn!(cancel_build)),
		],
	);

	app.post(
		"/repo/:repoId/build/:buildNum/restart",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission: permissions::workspace::ci::git_provider::repo::build::RESTART,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id = Uuid::parse_str(
						context.get_param(request_keys::WORKSPACE_ID).unwrap(),
					)
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
			},
			EveMiddleware::CustomFunction(pin_fn!(restart_build)),
		],
	);

	app.post(
		"/repo/:repoId/branch/:branchName/start",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::ci::git_provider::repo::build::START,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id = Uuid::parse_str(
						context.get_param(request_keys::WORKSPACE_ID).unwrap(),
					)
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
			},
			EveMiddleware::CustomFunction(pin_fn!(start_build_for_branch)),
		],
	);

	app.get(
		"/repo/:repoId/ref",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::ci::git_provider::repo::LIST,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id = Uuid::parse_str(
						context.get_param(request_keys::WORKSPACE_ID).unwrap(),
					)
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
			},
			EveMiddleware::CustomFunction(pin_fn!(list_git_ref_for_repo)),
		],
	);

	app.get(
		"/repo/:repoId/patr-ci-file/:gitRef",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::ci::git_provider::repo::INFO,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id = Uuid::parse_str(
						context.get_param(request_keys::WORKSPACE_ID).unwrap(),
					)
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
			},
			EveMiddleware::CustomFunction(pin_fn!(get_patr_ci_file)),
		],
	);

	app.post(
		"/repo/:repoId/patr-ci-file",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::ci::git_provider::repo::WRITE,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id = Uuid::parse_str(
						context.get_param(request_keys::WORKSPACE_ID).unwrap(),
					)
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
			},
			EveMiddleware::CustomFunction(pin_fn!(write_patr_ci_file)),
		],
	);

	app.delete(
		"/sign-out",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: false,
				permission:
					permissions::workspace::ci::git_provider::DISCONNECT,
				resource: closure_as_pinned_box!(|mut context| {
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
			},
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

	let user_id = context.get_token_data().unwrap().user_id().clone();

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
			.body(error!(INVALID_STATE_VALUE).to_string())?
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

	let github_provider_id = Uuid::new_v4();
	db::add_git_provider_to_workspace(
		context.get_database_connection(),
		&github_provider_id,
		&workspace_id,
		"github.com",
		GitProviderType::Github,
		Some(&login_name),
		Some(&access_token),
		&user_id,
	)
	.await?;

	context.success(GithubAuthCallbackResponse { github_provider_id });
	Ok(context)
}

async fn github_app_install(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let GithubAppInstallRequest {
		installation_id,
		git_provider_id,
		..
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	db::update_installation_id(
		context.get_database_connection(),
		&git_provider_id,
		&installation_id,
	)
	.await?;

	context.success(GithubAppInstallResponse {});
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

	let user_id = context.get_token_data().unwrap().user_id().clone();

	let config = context.get_state().config.clone();
	log::trace!("request_id: {request_id} - Syncing github repos for workspace {workspace_id}");

	let git_provider = db::get_git_provider_details_for_workspace_using_domain(
		context.get_database_connection(),
		&workspace_id,
		&user_id,
		"github.com",
	)
	.await?
	.status(500)?;

	let access_token = git_provider.access_token.status(500)?;

	if !git_provider.is_syncing {
		db::set_syncing(
			context.get_database_connection(),
			&git_provider.id,
			true,
			None,
		)
		.await?;
		service::queue_sync_github_repo(
			&git_provider.user_id,
			&git_provider.id,
			&request_id,
			access_token,
			git_provider.installation_id,
			&config,
		)
		.await?;
	}

	context.success(SyncReposResponse {});
	Ok(context)
}

async fn list_user_repositories(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let user_id = context.get_token_data().unwrap().user_id().clone();

	log::trace!("request_id: {request_id} - Listing github repos for workspace {workspace_id}");

	let git_provider = db::get_git_provider_details_for_workspace_using_domain(
		context.get_database_connection(),
		&workspace_id,
		&user_id,
		"github.com",
	)
	.await?
	.status(500)?;

	let repos =
		db::list_ci_repos_for_user(context.get_database_connection(), &user_id)
			.await?
			.into_iter()
			.map(|repo| RepositoryDetails {
				id: repo.git_provider_repo_uid,
				name: repo.repo_name,
				repo_owner: repo.repo_owner,
				clone_url: repo.clone_url,
			})
			.collect::<Vec<_>>();

	context.success(ListUserReposResponse { repos });
	Ok(context)
}

async fn list_workspace_repositories(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let user_id = context.get_token_data().unwrap().user_id().clone();

	log::trace!("request_id: {request_id} - Listing github repos for workspace {workspace_id}");

	let repos = db::list_ci_repos_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.into_iter()
	.map(|repo| WorkspaceRepositoryDetails {
		id: repo.git_provider_repo_uid,
		name: repo.repo_name,
		repo_owner: repo.repo_owner,
		clone_url: repo.clone_url,
		runner_id: repo.runner_id,
		activated: repo.activated,
	})
	.collect();

	context.success(ListWorkspaceReposResponse { repos });
	Ok(context)
}

async fn get_repo_info(
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

	context.success(GetRepoInfoResponse {
		repo: RepositoryDetails {
			id: repo.git_provider_repo_uid,
			name: repo.repo_name,
			repo_owner: repo.repo_owner,
			clone_url: repo.clone_url,
		},
	});

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

	db::get_git_provider_details_by_id(
		context.get_database_connection(),
		&repo.git_provider_id,
	)
	.await?
	.status(500)?;

	let ActivateRepoRequest { runner_id, .. } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let is_valid_runner = db::get_runners_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.into_iter()
	.any(|runner| runner.id == runner_id);

	if !is_valid_runner {
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	}

	db::activate_repo_for_workspace(
		context.get_database_connection(),
		&workspace_id,
		&repo_id,
		&runner_id,
	)
	.await?;

	context.success(ActivateRepoResponse {});
	Ok(context)
}

async fn add_repo_to_workspace(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let repo_id = context.get_param(request_keys::REPO_ID).unwrap().clone();

	log::trace!("request_id: {request_id} - Adding CI repo {repo_id} to workspace {workspace_id}");

	let repo = db::get_repo_details_using_github_uid(
		context.get_database_connection(),
		&workspace_id,
		&repo_id,
	)
	.await?
	.status(500)?;

	log::trace!("test 1");

	db::get_git_provider_details_by_id(
		context.get_database_connection(),
		&repo.git_provider_id,
	)
	.await?
	.status(500)?;

	log::trace!("test 2");

	let resource_id =
		db::generate_new_resource_id(context.get_database_connection()).await?;

	db::create_resource(
		context.get_database_connection(),
		&resource_id,
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::CI_REPO)
			.unwrap(),
		&workspace_id,
		&Utc::now(),
	)
	.await?;

	db::add_repo_for_workspace(
		context.get_database_connection(),
		&workspace_id,
		&resource_id,
		&repo.git_provider_repo_uid,
		&repo.git_provider_id,
		false,
	)
	.await?;

	context.success(AddRepoToWorkspaceResponse {});
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

	db::deactivate_workspace_repo(
		context.get_database_connection(),
		&workspace_id,
		&repo.git_provider_id,
		&repo.git_provider_repo_uid,
	)
	.await?;

	context.success(DeactivateRepoResponse {});
	Ok(context)
}

async fn delete_workspace_repo(
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

	let git_provider = db::get_git_provider_details_by_id(
		context.get_database_connection(),
		&repo.git_provider_id,
	)
	.await?
	.status(500)?;

	let workspace_repo = db::get_ci_repos_for_workspace(
		context.get_database_connection(),
		&workspace_id,
		&git_provider.id,
		&repo.git_provider_repo_uid,
	)
	.await?
	.status(404)?;

	db::delete_workspace_repo(
		context.get_database_connection(),
		&workspace_id,
		&repo.git_provider_id,
		&repo.git_provider_repo_uid,
		&Utc::now(),
	)
	.await?;

	db::delete_all_builds_for_workspace_repo(
		context.get_database_connection(),
		&workspace_repo.resource_id,
	)
	.await?;

	context.success(DeleteRepoResponse {});
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
		&repo.resource_id,
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
		&repo.resource_id,
		build_num,
	)
	.await?
	.status(400)
	.body("build not found")?;

	let steps = db::list_build_steps_for_build(
		context.get_database_connection(),
		&repo.resource_id,
		build_num,
	)
	.await?;

	context.success(GetBuildInfoResponse { build_info, steps });
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
		&repo.resource_id,
		build_num,
	)
	.await?
	.status(500)
	.body(error!(SERVER_ERROR).to_string())?;

	let build_step_id = BuildStepId {
		build_id: BuildId {
			repo_workspace_id: workspace_id,
			repo_id: repo.resource_id,
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

	// get the build status with lock, so that it won't be updated in rabbitmq
	// until this route ends.
	let build_status = db::get_build_status_for_update(
		context.get_database_connection(),
		&repo.resource_id,
		build_num,
	)
	.await?
	.status(400)
	.body("Build does not exists")?;

	if build_status == BuildStatus::Running ||
		build_status == BuildStatus::WaitingToStart
	{
		db::update_build_status(
			context.get_database_connection(),
			&repo.resource_id,
			build_num,
			BuildStatus::Cancelled,
		)
		.await?;
		db::update_build_finished_time(
			context.get_database_connection(),
			&repo.resource_id,
			build_num,
			&Utc::now(),
		)
		.await?;

		service::update_github_commit_status_for_build(
			context.get_database_connection(),
			&workspace_id,
			&repo.resource_id,
			build_num,
			CommitStatus::Errored,
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

	let git_provider = db::get_git_provider_details_by_id(
		context.get_database_connection(),
		&repo.git_provider_id,
	)
	.await?
	.status(500)?;
	let access_token = git_provider.access_token.status(500)?;

	let previous_build = db::get_build_details_for_build(
		context.get_database_connection(),
		&repo.resource_id,
		build_num,
	)
	.await?
	.status(500)?;

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
		.status(500)?;

		let pr_details = github_client
			.pulls()
			.get(&repo.repo_owner, &repo.repo_name, pull_number)
			.await
			.map_err(|err| {
				log::info!("error while getting pull request details: {err:#}");
				err
			})
			.ok()
			.status(500)?;

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
		return Error::as_result().status(500)?;
	};

	let ci_file_content = service::fetch_ci_file_content_from_github_repo(
		event_type.repo_owner(),
		event_type.repo_name(),
		event_type.commit_sha(),
		&access_token,
	)
	.await?;

	let build_num = service::create_build_for_repo(
		context.get_database_connection(),
		&repo.resource_id,
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
		ParseStatus::Error(err) => {
			db::update_build_status(
				context.get_database_connection(),
				&repo.resource_id,
				build_num,
				BuildStatus::Errored,
			)
			.await?;
			db::update_build_message(
				context.get_database_connection(),
				&repo.resource_id,
				build_num,
				&err,
			)
			.await?;
			db::update_build_finished_time(
				context.get_database_connection(),
				&repo.resource_id,
				build_num,
				&Utc::now(),
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
		Ok(works) => match works {
			service::EvaluationStatus::Success(works) => works,
			service::EvaluationStatus::Error(err) => {
				db::update_build_status(
					context.get_database_connection(),
					&repo.resource_id,
					build_num,
					BuildStatus::Errored,
				)
				.await?;
				db::update_build_message(
					context.get_database_connection(),
					&repo.resource_id,
					build_num,
					&err,
				)
				.await?;
				db::update_build_finished_time(
					context.get_database_connection(),
					&repo.resource_id,
					build_num,
					&Utc::now(),
				)
				.await?;
				return Ok(context);
			}
		},
		Err(err) => {
			log::info!("request_id: {request_id} - Error while evaluating ci work steps {err:#?}");
			db::update_build_status(
				context.get_database_connection(),
				&repo.resource_id,
				build_num,
				BuildStatus::Errored,
			)
			.await?;
			db::update_build_finished_time(
				context.get_database_connection(),
				&repo.resource_id,
				build_num,
				&Utc::now(),
			)
			.await?;
			return Ok(context);
		}
	};

	service::add_build_steps_in_db(
		context.get_database_connection(),
		&repo.resource_id,
		build_num,
		&works,
		&request_id,
	)
	.await?;

	service::update_github_commit_status_for_build(
		context.get_database_connection(),
		&git_provider.workspace_id,
		&repo.resource_id,
		build_num,
		CommitStatus::Running,
	)
	.await?;

	context.commit_database_transaction().await?;

	service::queue_check_and_start_ci_build(
		BuildId {
			repo_workspace_id: git_provider.workspace_id,
			repo_id: repo.resource_id.clone(),
			build_num,
		},
		pipeline.services,
		works,
		event_type,
		&context.get_state().config,
		&request_id,
	)
	.await?;

	context.success(RestartBuildResponse {
		build_num: build_num as u64,
	});
	Ok(context)
}

async fn start_build_for_branch(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let repo_id = context.get_param(request_keys::REPO_ID).unwrap().clone();
	let branch_name = context
		.get_param(request_keys::BRANCH_NAME)
		.unwrap()
		.clone();

	log::trace!("request_id: {request_id} - Starting build for repo {repo_id} at branch {branch_name}");

	let repo = db::get_repo_details_using_github_uid_for_workspace(
		context.get_database_connection(),
		&workspace_id,
		&repo_id,
	)
	.await?
	.status(500)?;

	let git_provider = db::get_git_provider_details_by_id(
		context.get_database_connection(),
		&repo.git_provider_id,
	)
	.await?
	.status(500)?;

	let access_token = git_provider.access_token.status(500)?;

	let github_client =
		octorust::Client::new("patr", Credentials::Token(access_token.clone()))
			.map_err(|err| {
				log::info!("error while octorust init: {err:#}");
				err
			})
			.ok()
			.status(500)?;

	let github_branch = github_client
		.repos()
		.get_branch(&repo.repo_owner, &repo.repo_name, &branch_name)
		.await
		.map_err(|err| {
			log::info!("error while getting webhooks list: {err:#}");
			err
		})
		.ok()
		.status(500)?;

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

	let build_num = service::create_build_for_repo(
		context.get_database_connection(),
		&repo.resource_id,
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
		ParseStatus::Error(err) => {
			db::update_build_status(
				context.get_database_connection(),
				&repo.resource_id,
				build_num,
				BuildStatus::Errored,
			)
			.await?;
			db::update_build_message(
				context.get_database_connection(),
				&repo.resource_id,
				build_num,
				&err,
			)
			.await?;
			db::update_build_finished_time(
				context.get_database_connection(),
				&repo.resource_id,
				build_num,
				&Utc::now(),
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
		Ok(works) => match works {
			service::EvaluationStatus::Success(works) => works,
			service::EvaluationStatus::Error(err) => {
				db::update_build_status(
					context.get_database_connection(),
					&repo.resource_id,
					build_num,
					BuildStatus::Errored,
				)
				.await?;
				db::update_build_message(
					context.get_database_connection(),
					&repo.resource_id,
					build_num,
					&err,
				)
				.await?;
				db::update_build_finished_time(
					context.get_database_connection(),
					&repo.resource_id,
					build_num,
					&Utc::now(),
				)
				.await?;
				return Ok(context);
			}
		},
		Err(err) => {
			log::info!("request_id: {request_id} - Error while evaluating ci work steps {err:#?}");
			db::update_build_status(
				context.get_database_connection(),
				&repo.resource_id,
				build_num,
				BuildStatus::Errored,
			)
			.await?;
			db::update_build_finished_time(
				context.get_database_connection(),
				&repo.resource_id,
				build_num,
				&Utc::now(),
			)
			.await?;
			return Ok(context);
		}
	};

	service::add_build_steps_in_db(
		context.get_database_connection(),
		&repo.resource_id,
		build_num,
		&works,
		&request_id,
	)
	.await?;

	context.commit_database_transaction().await?;

	service::update_github_commit_status_for_build(
		context.get_database_connection(),
		&git_provider.workspace_id,
		&repo.resource_id,
		build_num,
		CommitStatus::Running,
	)
	.await?;

	service::queue_check_and_start_ci_build(
		BuildId {
			repo_workspace_id: git_provider.workspace_id,
			repo_id: repo.resource_id.clone(),
			build_num,
		},
		pipeline.services,
		works,
		event_type,
		&context.get_state().config,
		&request_id,
	)
	.await?;

	context.success(RestartBuildResponse {
		build_num: build_num as u64,
	});
	Ok(context)
}

async fn list_git_ref_for_repo(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let repo_id = context.get_param(request_keys::REPO_ID).unwrap().clone();

	log::trace!(
		"request_id: {request_id} - Fetching all git ref for {repo_id}"
	);

	let repo = db::get_repo_details_using_github_uid_for_workspace(
		context.get_database_connection(),
		&workspace_id,
		&repo_id,
	)
	.await?
	.status(500)?;

	let git_provider = db::get_git_provider_details_by_id(
		context.get_database_connection(),
		&repo.git_provider_id,
	)
	.await?
	.status(500)?;
	let (_login_name, access_token) = git_provider
		.login_name
		.zip(git_provider.access_token)
		.status(500)?;

	let refs = service::list_git_ref_for_repo(
		&repo.repo_owner,
		&repo.repo_name,
		&access_token,
	)
	.await?;

	context.success(ListGitRefForRepoResponse { refs });
	Ok(context)
}

async fn get_patr_ci_file(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let repo_id = context.get_param(request_keys::REPO_ID).unwrap().clone();
	let git_ref = context.get_param(request_keys::GIT_REF).unwrap().clone();

	log::trace!("request_id: {request_id} - Fetching CI file for {repo_id} at ref {git_ref}");

	let repo = db::get_repo_details_using_github_uid_for_workspace(
		context.get_database_connection(),
		&workspace_id,
		&repo_id,
	)
	.await?
	.status(500)?;

	let git_provider = db::get_git_provider_details_by_id(
		context.get_database_connection(),
		&repo.git_provider_id,
	)
	.await?
	.status(500)?;
	let (_login_name, access_token) = git_provider
		.login_name
		.zip(git_provider.access_token)
		.status(500)?;

	let ci_file_content = service::fetch_ci_file_content_from_github_repo(
		&repo.repo_owner,
		&repo.repo_name,
		&git_ref,
		&access_token,
	)
	.await?;

	context.success(GetPatrCiFileResponse {
		file_content: Base64String::from(ci_file_content),
	});
	Ok(context)
}

async fn write_patr_ci_file(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let repo_id = context.get_param(request_keys::REPO_ID).unwrap().clone();

	log::trace!(
		"request_id: {request_id} - Writing patr ci fiile to repo {repo_id}"
	);

	let WritePatrCiFileRequest {
		commit_message,
		parent_commit_sha,
		branch_name,
		ci_file_content,
		..
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let repo = db::get_repo_details_using_github_uid_for_workspace(
		context.get_database_connection(),
		&workspace_id,
		&repo_id,
	)
	.await?
	.status(500)?;

	let git_provider = db::get_git_provider_details_by_id(
		context.get_database_connection(),
		&repo.git_provider_id,
	)
	.await?
	.status(500)?;

	let (_login_name, access_token) = git_provider
		.login_name
		.zip(git_provider.access_token)
		.status(500)?;

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

	context.success(WritePatrCiFileResponse {});
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

	let user_id = context.get_token_data().unwrap().user_id().clone();

	log::trace!("request_id: {request_id} - Signout github from patr for workspace {workspace_id}");

	let git_provider = db::get_git_provider_details_for_workspace_using_domain(
		context.get_database_connection(),
		&workspace_id,
		&user_id,
		"github.com",
	)
	.await?
	.status(500)?;

	db::remove_git_provider_credentials(
		context.get_database_connection(),
		&git_provider.id,
	)
	.await?;

	context.success(GithubSignOutResponse {});
	Ok(context)
}
