use api_macros::closure_as_pinned_box;
use api_models::{
	models::workspace::ci::github::{
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
		GithubAuthCallbackRequest,
		GithubAuthResponse,
		GithubListRepos,
		GithubListReposResponse,
		GithubSignOutResponse,
		RestartBuildInfo,
		RestartBuildRequest,
		RestartBuildResponse,
	},
	utils::Uuid,
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use hyper::{body::HttpBody, header::COOKIE, Request};
use reqwest::header::AUTHORIZATION;
use serde_json::Value;

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

	app.delete(
		"/sign-out",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::github::auth::DELETE,
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

	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

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

	let session_cookie = response.headers().get_all("set-cookie");

	let mut cookie = session_cookie.iter();
	let _oauth = cookie.next();
	let session = cookie.next();
	let session = if let Some(session) = session {
		session.to_str()?
	} else {
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string());
	};

	let split = session.split(';');
	let vec = split.collect::<Vec<&str>>();

	let uri = format!("{}/api/user", config.drone.url);
	let response = hyper::Client::new()
		.request(
			Request::builder()
				.method("GET")
				.uri(&uri)
				.header(COOKIE, vec[0])
				.body(hyper::Body::empty())?,
		)
		.await?;

	let mut body = response.into_body();
	let mut buffer = String::new();

	while let Some(chunk) = body.data().await {
		buffer.push_str(&String::from_utf8(chunk?.to_vec())?);
	}

	let json_body: Value = serde_json::from_str(&buffer)?;
	let drone_username = json_body.get("login");
	let drone_username =
		if let Some(Value::String(drone_username)) = drone_username {
			drone_username.clone()
		} else {
			return Error::as_result()
				.status(500)
				.body(error!(SERVER_ERROR).to_string());
		};

	db::add_drone_username_to_workspace(
		context.get_database_connection(),
		&workspace_id,
		&drone_username,
	)
	.await?;

	Ok(context)
}

async fn list_repositories(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let config = context.get_state().config.clone();

	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let drone_username = db::get_drone_username(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.status(404)
	.body(error!(USER_NOT_FOUND).to_string())?;

	// acquire new connection
	let user_hash = db::get_drone_access_token(
		context.get_database_connection(),
		&drone_username,
	)
	.await?
	.status(500)
	.body(error!(SERVER_ERROR).to_string())?;

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

	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let drone_username = db::get_drone_username(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.status(404)
	.body(error!(USER_NOT_FOUND).to_string())?;

	// acquire new connection
	let user_hash = db::get_drone_access_token(
		context.get_database_connection(),
		&drone_username,
	)
	.await?
	.status(500)
	.body(error!(SERVER_ERROR).to_string())?;

	let ActivateGithubRepoRequest { repo_name, .. } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let client = reqwest::Client::new();
	let response = client
		.post(format!(
			"{}/api/repos/{}/{}",
			config.drone.url, drone_username, repo_name
		))
		.header(AUTHORIZATION, format!("Bearer {}", user_hash))
		.send()
		.await?;

	if response.status() != 200 {
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string());
	}

	let activated_repo = response.json::<ActivateGithubRepoResponse>().await?;

	context.success(activated_repo);

	Ok(context)
}

async fn get_build_list(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let config = context.get_state().config.clone();

	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let drone_username = db::get_drone_username(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.status(404)
	.body(error!(USER_NOT_FOUND).to_string())?;

	// acquire new connection
	let user_hash = db::get_drone_access_token(
		context.get_database_connection(),
		&drone_username,
	)
	.await?
	.status(500)
	.body(error!(SERVER_ERROR).to_string())?;

	let GetBuildListRequest { repo_name, .. } = context
		.get_query_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let client = reqwest::Client::new();
	let response = client
		.get(format!(
			"{}/api/repos/{}/{}/builds",
			config.drone.url, drone_username, repo_name
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

	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let drone_username = db::get_drone_username(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.status(404)
	.body(error!(USER_NOT_FOUND).to_string())?;

	// acquire new connection
	let user_hash = db::get_drone_access_token(
		context.get_database_connection(),
		&drone_username,
	)
	.await?
	.status(500)
	.body(error!(SERVER_ERROR).to_string())?;

	let GetBuildInfoRequest {
		repo_name,
		build_num,
		..
	} = context
		.get_query_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let client = reqwest::Client::new();
	let response = client
		.get(format!(
			"{}/api/repos/{}/{}/builds/{}",
			config.drone.url, drone_username, repo_name, build_num
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
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let drone_username = db::get_drone_username(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.status(404)
	.body(error!(USER_NOT_FOUND).to_string())?;

	// acquire new connection
	let user_hash = db::get_drone_access_token(
		context.get_database_connection(),
		&drone_username,
	)
	.await?
	.status(500)
	.body(error!(SERVER_ERROR).to_string())?;

	let GetBuildLogRequest {
		repo_name,
		build_num,
		stage,
		step,
		..
	} = context
		.get_query_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let client = reqwest::Client::new();
	let response = client
		.get(format!(
			"{}/api/repos/{}/{}/builds/{}/logs/{}/{}",
			config.drone.url, drone_username, repo_name, build_num, stage, step
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

	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let drone_username = db::get_drone_username(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.status(404)
	.body(error!(USER_NOT_FOUND).to_string())?;

	// acquire new connection
	let user_hash = db::get_drone_access_token(
		context.get_database_connection(),
		&drone_username,
	)
	.await?
	.status(500)
	.body(error!(SERVER_ERROR).to_string())?;
	let RestartBuildRequest {
		repo_name,
		build_num,
		..
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let client = reqwest::Client::new();
	let response = client
		.post(format!(
			"{}/api/repos/{}/{}/builds/{}",
			config.drone.url, drone_username, repo_name, build_num
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

async fn sign_out(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	if db::get_drone_username(context.get_database_connection(), &workspace_id)
		.await?
		.is_some()
	{
		db::delete_user_by_login(
			context.get_database_connection(),
			&workspace_id,
		)
		.await?;

		context.success(GithubSignOutResponse {});
		Ok(context)
	} else {
		Error::as_result()
			.status(404)
			.body(error!(USER_NOT_FOUND).to_string())
	}
}
