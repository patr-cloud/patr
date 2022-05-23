use api_macros::closure_as_pinned_box;
use api_models::{
	models::workspace::ci::github::{
		GithubAuthCallbackRequest,
		ActivateGithubRepoRequest,
		ActivateGithubRepoResponse,
		BuildInfo,
		BuildList,
		BuildLogs,
		GetBuildInfoRequest,
		GetBuildInfoResponse,
		GetBuildListRequest,
		GetBuildListResponse,
		GetBuildLogRequest,
		GetBuildLogResponse,
		GithubAuthCallbackResponse,
		GithubAuthResponse,
		GithubListRepos,
		GithubListReposResponse,
		RestartBuildInfo,
		RestartBuildRequest,
		RestartBuildResponse,
	},
	utils::Uuid,
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use reqwest::header::{AUTHORIZATION, COOKIE};

use crate::{
	app::{create_eve_app, App},
	db,
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

	app.get(
		"/oauth-callback",
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
		"/list-repositories",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::github::auth::CREATE,
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
		"/repo/activate",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::github::auth::CREATE,
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

	app.get(
		"/repo/build-logs",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::github::auth::CREATE,
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

	app.get(
		"/repo/build-list",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::github::auth::CREATE,
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
		"/repo/build-info",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::github::auth::CREATE,
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

	app.post(
		"/repo/restart-build",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::github::auth::CREATE,
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

	app
}

async fn connect_to_github(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let config = context.get_state().config.clone();

	let client = reqwest::Client::builder()
		.redirect(reqwest::redirect::Policy::none())
		.build()?;
	let response = client
		.get(format!("{}/login", config.drone.url))
		.send()
		.await?;
	let oauth_url = response
		.headers()
		.get(reqwest::header::LOCATION)
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;
	let oauth_url = oauth_url.to_str()?;

	context.success(GithubAuthResponse {
		oauth_url: oauth_url.to_string(),
	});
	Ok(context)
}

async fn github_oauth_callback(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let config = context.get_state().config.clone();

	let GithubAuthCallbackRequest { code, state, .. } = context
		.get_query_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let response = reqwest::Client::new()
		.get(format!(
			"{}/login?code={}&state={}",
			config.drone.url, code, state
		))
		.header(COOKIE, format!("_oauth_state_={}", state))
		.send()
		.await?;
	if response.status() != 200 {
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string());
	}

	// TODO - get oauth_access_token and user_hash from the db and send it to
	// frontend

	context.success(GithubAuthCallbackResponse {});
	Ok(context)
}

async fn list_repositories(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let config = context.get_state().config.clone();

	let user_hash = context
		.get_request()
		.get_query()
		.get(request_keys::TOKEN)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let client = reqwest::Client::new();
	let response = client
		.get(format!("{}/api/user/repos", config.drone.url))
		.header(AUTHORIZATION, format!("Bearer {}", user_hash))
		.send()
		.await?;

	if response.status() != 200 {
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string());
	}

	let repos = response.json::<Vec<GithubListRepos>>().await?;

	context.success(GithubListReposResponse { repos });

	Ok(context)
}

async fn activate_repo(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let config = context.get_state().config.clone();

	let _workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let user_hash = context
		.get_request()
		.get_query()
		.get(request_keys::TOKEN)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let ActivateGithubRepoRequest { owner, name, .. } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let client = reqwest::Client::new();
	let response = client
		.post(format!("{}/api/repos/{}/{}", config.drone.url, owner, name))
		.header(AUTHORIZATION, format!("Bearer {}", user_hash))
		.send()
		.await?;

	if response.status() != 200 {
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string());
	}

	let activated_repo = response.json::<ActivateGithubRepoResponse>().await?;

	// TODO - add useful information to workspace table
	// Like - repo_id, repo_name, repo_owner, repo_url, activated(bool)

	context.success(activated_repo);

	Ok(context)
}

async fn get_build_list(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let config = context.get_state().config.clone();

	let user_hash = context
		.get_request()
		.get_query()
		.get(request_keys::TOKEN)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let GetBuildListRequest { owner, name, .. } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let client = reqwest::Client::new();
	let response = client
		.get(format!(
			"{}/api/repos/{}/{}/builds",
			config.drone.url, owner, name
		))
		.header(AUTHORIZATION, format!("Bearer {}", user_hash))
		.send()
		.await?;

	if response.status() != 200 {
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string());
	}

	let builds = response.json::<Vec<BuildList>>().await?;

	context.success(GetBuildListResponse { builds });

	Ok(context)
}

async fn get_build_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let config = context.get_state().config.clone();

	let user_hash = context
		.get_request()
		.get_query()
		.get(request_keys::TOKEN)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let GetBuildInfoRequest {
		owner, name, build, ..
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let client = reqwest::Client::new();
	let response = client
		.get(format!(
			"{}/api/repos/{}/{}/builds/{}",
			config.drone.url, owner, name, build
		))
		.header(AUTHORIZATION, format!("Bearer {}", user_hash))
		.send()
		.await?;

	if response.status() != 200 {
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string());
	}

	let build_info = response.json::<BuildInfo>().await?;

	context.success(GetBuildInfoResponse { build_info });

	Ok(context)
}

async fn get_build_logs(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let config = context.get_state().config.clone();

	let user_hash = context
		.get_request()
		.get_query()
		.get(request_keys::TOKEN)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let GetBuildLogRequest {
		owner,
		name,
		build,
		stage,
		step,
		..
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let client = reqwest::Client::new();
	let response = client
		.get(format!(
			"{}/api/repos/{}/{}/builds/{}/logs/{}/{}",
			config.drone.url, owner, name, build, stage, step
		))
		.header(AUTHORIZATION, format!("Bearer {}", user_hash))
		.send()
		.await?;

	if response.status() != 200 {
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string());
	}

	let build_logs = response.json::<Vec<BuildLogs>>().await?;

	context.success(GetBuildLogResponse { build_logs });

	Ok(context)
}

async fn restart_build(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let config = context.get_state().config.clone();

	let user_hash = context
		.get_request()
		.get_query()
		.get(request_keys::TOKEN)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let RestartBuildRequest {
		owner, name, build, ..
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let client = reqwest::Client::new();
	let response = client
		.post(format!(
			"{}/api/repos/{}/{}/builds/{}",
			config.drone.url, owner, name, build
		))
		.header(AUTHORIZATION, format!("Bearer {}", user_hash))
		.send()
		.await?;

	if response.status() != 200 {
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string());
	}

	let restart_build_info = response.json::<RestartBuildInfo>().await?;

	context.success(RestartBuildResponse { restart_build_info });

	Ok(context)
}
