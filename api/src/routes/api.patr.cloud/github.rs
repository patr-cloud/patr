use api_macros::closure_as_pinned_box;
use api_models::{models::workspace::get_github_info::*, utils::Uuid};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use octocrab::{models::repos::GitUser, Octocrab};

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
		"/get-repository",
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
		"/config-static-build",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::github::action::CREATE,
				closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						"ed413db9bc214a84a273874c487a3b23";
					// context.get_param(request_keys::WORKSPACE_ID).unwrap();
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
		"/config-deployment-build",
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

async fn list_github_repos(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - requested to get repo", request_id,);

	let GetGithubRepoRequest {
		access_token,
		owner_name,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let client = reqwest::Client::new();

	let response = client
		.get(format!("https://api.github.com/users/{}/repos", owner_name))
		.header("AUTHORIZATION", format!("token {}", access_token))
		.send()
		.await?;

	// TODO - send response.body as a response
	println!("{:#?}", response);

	Ok(context)
}

async fn configure_github_build_steps_static_site(
	context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	log::trace!(
		"request_id: {} - requested config build steps using github actions",
		request_id,
	);

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
		)
		.await?;
	} else {
		service::github_actions_for_vanilla_static_site(
			access_token,
			owner_name,
			repo_name,
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

	let octocrab = Octocrab::builder()
		.personal_token(access_token.clone())
		.build()?;
	let client = reqwest::Client::new();

	let response = client
		.get(format!("https://api.github.com/repos/{}/{}/contents/.github/workflow/build.yaml", owner_name, repo_name))
		.header("AUTHORIZATION", format!("token {}", access_token))
		.send()
		.await?;

	println!("{:#?}", response);

	if framework == "node" {
		if response.status() == 404 {
			octocrab
				.repos(owner_name, repo_name)
				.create_file(
					".github/workflows/build.yaml",
					"created: build.yaml",
					format!(
						r#"
name: Github action for your deployment

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

- name: build docker image from Dockerfile
    run: |
    docker build ./ -t <tag-todo-ideally-should-be-commit-hash-8-char>
    echo TODO
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
		} else if response.status() == 200 {
			octocrab
				.repos(owner_name, repo_name)
				.create_file(
					".github/workflows/build.yaml",
					"updated: build.yaml",
					format!(
						r#"
name: Github action for your deployment

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

- name: build docker image from Dockerfile
    run: |
    docker build ./ -t <tag-todo-ideally-should-be-commit-hash-8-char>
    echo TODO
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
	}

	Ok(context)
}
