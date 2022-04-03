use api_macros::closure_as_pinned_box;
use api_models::{models::workspace::get_github_info::*, utils::Uuid};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use reqwest::header::{AUTHORIZATION, USER_AGENT};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac::permissions,
	pin_fn,
	service,
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
		"/accessToken/:code",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(get_access_token)),
		],
	);

	app.get(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::github::repo::LIST,
				closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
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
			EveMiddleware::CustomFunction(pin_fn!(list_github_repos)),
		],
	);
	app.post(
		"/configure-static-build",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::github::action::CREATE,
				closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
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
			EveMiddleware::CustomFunction(pin_fn!(
				configure_github_build_steps_static_site
			)),
		],
	);
	app.post(
		"/configure-deployment-build",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::github::action::CREATE,
				closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
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
			EveMiddleware::CustomFunction(pin_fn!(
				configure_github_build_steps_deployment
			)),
		],
	);

	app
}

async fn get_access_token(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let code = context.get_param(request_keys::CODE).unwrap();
	let config = context.get_state().config.clone();
	let client_id = config.github.client_id;
	let client_secret = config.github.client_secret;

	let client = reqwest::Client::new();

	let response = client.post(format!("https://github.com/login/oauth/access_token?client_id={}&client_secret={}&code={}", client_id, client_secret, code)).send().await?.text().await?;

	if response.contains("access_token") {
		let access_token_raw: Vec<&str> = response.split('&').collect();
		let token: Vec<&str> = access_token_raw[0].split('=').collect();

		context.success(GithubAccessToken {
			access_token: token[1].to_string(),
		});

		Ok(context)
	} else {
		// Cannot tell the user about the message
		// As user has no power to do anything with this message
		// Therefore sending 500 error
		log::error!("verification error : {}", response);
		Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())
	}
}

async fn list_github_repos(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let GetGithubRepoRequest {
		access_token,
		owner_name,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let user_agent = context
		.get_header("User-Agent")
		.unwrap_or_else(|| owner_name.clone());

	let client = reqwest::Client::new();

	let response = client
		.get("https://api.github.com/user/repos")
		.header(AUTHORIZATION, format!("Token {}", access_token))
		.header(USER_AGENT, user_agent)
		.send()
		.await?;

	let response = response.json::<Vec<GithubResponse>>().await?;

	context.success(GetGithubRepoResponse { response });

	Ok(context)
}

async fn configure_github_build_steps_static_site(
	context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let GetGithubStaticSiteBuildStepRequest {
		access_token,
		owner_name,
		repo_name,
		framework, // node, or for now nothing else
		build_command,
		publish_dir,
		node_version,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let user_agent = context
		.get_header("User-Agent")
		.unwrap_or_else(|| owner_name.clone());

	// for now only going forward with node and vanilla
	// will add other frameworks support
	if framework == "node" {
		service::github_actions_for_node_static_site(
			access_token,
			owner_name,
			repo_name,
			build_command,
			publish_dir,
			node_version,
			user_agent,
		)
		.await?;
	} else {
		service::github_actions_for_vanilla_static_site(
			access_token,
			owner_name,
			repo_name,
			user_agent,
		)
		.await?
	}

	Ok(context)
}

async fn configure_github_build_steps_deployment(
	context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	log::trace!(
		"request_id: {} - requested config build steps using github actions",
		request_id,
	);

	let GetGithubDeploymentBuildStepRequest {
		access_token,
		owner_name,
		repo_name,
		build_command,
		publish_dir,
		node_version,
		framework,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let user_agent = context
		.get_header("User-Agent")
		.unwrap_or_else(|| owner_name.clone());

	if framework == "node" {
		service::github_actions_for_node_deployment(
			access_token,
			owner_name,
			repo_name,
			build_command,
			publish_dir,
			node_version,
			user_agent,
		)
		.await?;
	}
	Ok(context)
}
