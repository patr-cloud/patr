use api_macros::closure_as_pinned_box;
use api_models::{models::workspace::get_github_info::*, utils::Uuid};
use eve_rs::{App as EveApp, AsError, NextHandler, Context};
use octocrab::{models::repos::GitUser, Octocrab};

use crate::{
	app::{create_eve_app, App},
	db, error,
	models::rbac::permissions,
	pin_fn,
	utils::{
		constants::request_keys, Error, ErrorData, EveContext, EveMiddleware
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
			EveMiddleware::CustomFunction(pin_fn!(get_github_repo)),
		],
	);
	app.post(
		"/config-static-build",
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

async fn get_github_repo(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - requested to get repo", request_id,);

	let GetGithubRepoRequest {
		access_token,
		owner_name,
		repo_name,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let octocrab = Octocrab::builder().personal_token(access_token).build()?;
	// let octocrab = Octocrab::builder().build()?;

	let repo = octocrab.repos(&owner_name, &repo_name).get().await?;

	let html_url = match repo.html_url {
		Some(url) => url.to_string(),
		None => String::from("_"),
	};

	context.success(GetGithubRepoResponse {
		id: repo.id.to_string(),
		name: repo.name,
		html_url,
	});

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

	let octocrab = Octocrab::builder().personal_token(access_token).build()?;

	if framework == "node" {
		octocrab
			.repos(&owner_name, &repo_name)
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
	} else {
		octocrab
			.repos(&owner_name, &repo_name)
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
				    steps:
				    - uses: actions/checkout@master
				    - name: Archive Release
					    uses: {owner_name}/{repo_name}@master
					    with:
					    type: 'zip'
					    filename: 'release.zip'
				    - name: push to patr
				      run: echo TODO
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
	context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	// create a request_id
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

	let octocrab = Octocrab::builder().personal_token(access_token).build()?;

	if framework == "node" {
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
	}

	Ok(context)
}
