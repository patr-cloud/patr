use api_models::utils::Uuid;
use eve_rs::{App as EveApp, AsError, NextHandler};
use octocrab::{models::repos::GitUser, Octocrab};

use crate::{
	app::{create_eve_app, App},
	error,
	pin_fn,
	utils::{Error, ErrorData, EveContext, EveMiddleware},
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
		"/get-repository",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(get_github_repo)),
		],
	);
	app.post(
		"/config-static-build",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(
				configure_github_build_steps_static_site
			)),
		],
	);
	app.post(
		"/config-deployment-build",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(
				configure_github_build_steps_deployment
			)),
		],
	);

	app
}

async fn get_github_repo(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - requested to get repo", request_id,);

	// take our useful info(access_token, user_name, repo_name) from the request
	// body create a api_model named as GetGithubRepoRequest
	let GetGithubRepoRequest {
		access_token,
		owner_name,
		repo_name,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let octocrab = Octocrab::builder().personal_token(access_token).build()?;

	let repo = octocrab.repos(owner_name, repo_name).get().await?;

	// TODO - check for successful/error octocrab api response and send the
	// desired response as Repository struct has 84 fields

	Ok(context)
}

async fn configure_github_build_steps_static_site(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	log::trace!(
		"request_id: {} - requested config build steps using github actions",
		request_id,
	);

	// create a api_model named as GithubStaticSiteBuildStepRequest
	// That contains a all the information needed for the request
	// For normal projects the publish_dir should be "." and build command
	// should be empty string("")
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

	// request github with Authorizarion header setup with access_token
	// setup header with acccess_token
	let octocrab = Octocrab::builder().personal_token(access_token).build()?;

	if framework == "node" {
		octocrab
			.repos(owner_name, repo_name)
			.create_file(
				".github/workflows/build.yaml",
				"created: build.yaml",
				format!(
					r#"
				name: Github action for your static site

				on:
			      push:
				    branch: [main]
				
				jobs:
				  build:

				  	runs-on: ubuntu-latest

					strategy:
					  matrix: 
					    node-version: {node_version}
					
				steps:
				- uses: actions/checkout@v2
				- name: using node ${{matrix.node-version}}
			      uses: actions/setup-node@v2
				  with: 
				  	node-version: ${{matrix.node-version}}
					cache: 'npm'
				- run: npm install
				- run: {build_command}
				- run: npm run test --if-present
			"#
				),
			)
			.branch("main")
			.commiter(GitUser {
				name: "Patr Configuration".to_string(),
				email: "hello@patr.cloud".to_string(),
			})
			.author(GitUser {
				name: "Patr Configuration".to_string(),
				email: "hello@patr.cloud".to_string(),
			})
			.send()
			.await?;
	}

	Ok(context)
}

async fn configure_github_build_steps_deployment(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	// create a request_id
	let request_id = Uuid::new_v4();
	log::trace!(
		"request_id: {} - requested config build steps using github actions",
		request_id,
	);

	// take our useful info(access_token, user_name, repo_name, build_command,
	// publish_dir) from the request body create a api_model named as
	// GithubDeploymentBuildStepRequest
	let GetGithubDeploymentBuildStepRequest {
		access_token,
		owner_name,
		repo_name,
		build_command,
		publish_dir,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	// create a github actions config file based on the the deployment steps

	// setup header with acccess_token
	// example - curl -i -H "Authorization: <access_token>" https://api.github.com/repos/{owner}/{repo_name}
	// using {reqwest crate}
	// if response is success send the repository back to user
	// else send an appropriate error message as  response

	Ok(context)
}

// route to setup github repo with github actions
