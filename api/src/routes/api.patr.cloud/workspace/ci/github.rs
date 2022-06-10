use api_macros::closure_as_pinned_box;
use api_models::{
	models::workspace::ci::github::{
		ActivateGithubRepoResponse,
		BuildDetails,
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
	},
	utils::Uuid,
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::{ci::DroneUserInfoResponse, rbac::permissions},
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

	app
}

async fn connect_to_github(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let response = reqwest::Client::builder()
		.redirect(reqwest::redirect::Policy::none())
		.build()?
		.get(format!("{}/login", context.get_state().config.drone.url))
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
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let GithubAuthCallbackRequest { code, state, .. } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let response = reqwest::Client::builder()
		.redirect(reqwest::redirect::Policy::none())
		.build()?
		.get(format!("{}/login", context.get_state().config.drone.url))
		.header(reqwest::header::COOKIE, format!("_oauth_state_={}", state))
		.query(&[("code", code), ("state", state)])
		.send()
		.await?
		.error_for_status()?;

	let cookie = response
		.cookies()
		.find(|cookie| cookie.name() == "_session_")
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let response = reqwest::Client::new()
		.post(format!(
			"{}/api/user/token",
			context.get_state().config.drone.url
		))
		.header(
			reqwest::header::COOKIE,
			format!("{}={}", cookie.name(), cookie.value()),
		)
		.send()
		.await?
		.error_for_status()?
		.json::<DroneUserInfoResponse>()
		.await?;

	db::set_drone_username_and_token_for_workspace(
		context.get_database_connection(),
		&workspace_id,
		&response.login,
		&response.token,
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

	let (_, drone_token) = db::get_drone_username_and_token_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.status(404)
	.body(error!(USER_NOT_FOUND).to_string())?;

	let repos = reqwest::Client::new()
		.get(format!(
			"{}/api/user/repos",
			context.get_state().config.drone.url
		))
		.bearer_auth(drone_token)
		.send()
		.await?
		.error_for_status()?
		.json()
		.await?;

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

	let repo_owner = context.get_param(request_keys::REPO_OWNER).unwrap().clone();
	let repo_name = context.get_param(request_keys::REPO_NAME).unwrap().clone();

	let (_, drone_token) = db::get_drone_username_and_token_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.status(404)
	.body(error!(USER_NOT_FOUND).to_string())?;

	reqwest::Client::new()
		.post(format!(
			"{}/api/repos/{}/{}",
			context.get_state().config.drone.url,
			repo_owner,
			repo_name
		))
		.bearer_auth(drone_token)
		.send()
		.await?
		.error_for_status()?;

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

	let repo_owner = context.get_param(request_keys::REPO_OWNER).unwrap().clone();
	let repo_name = context.get_param(request_keys::REPO_NAME).unwrap().clone();

	let (_, drone_token) = db::get_drone_username_and_token_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.status(404)
	.body(error!(USER_NOT_FOUND).to_string())?;

	reqwest::Client::new()
		.delete(format!(
			"{}/api/repos/{}/{}",
			context.get_state().config.drone.url,
			repo_owner,
			repo_name
		))
		.bearer_auth(drone_token)
		.send()
		.await?
		.error_for_status()?;

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

	let repo_owner = context.get_param(request_keys::REPO_OWNER).unwrap().clone();
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

	let repo_owner = context.get_param(request_keys::REPO_OWNER).unwrap().clone();
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

	let repo_owner = context.get_param(request_keys::REPO_OWNER).unwrap().clone();
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

	let logs = reqwest::Client::new()
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
		.await?
		.error_for_status()?
		.json()
		.await?;

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

	let repo_owner = context.get_param(request_keys::REPO_OWNER).unwrap().clone();
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

	db::remove_drone_username_and_token_from_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?;

	context.success(GithubSignOutResponse {});
	Ok(context)
}
