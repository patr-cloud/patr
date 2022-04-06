use eve_rs::AsError;
use octocrab::{models::repos::GitUser, Octocrab};
use reqwest::header::{AUTHORIZATION, USER_AGENT};

use crate::{error, models::db_mapping::GithubResponseBody, utils::Error};

pub async fn github_actions_for_node_static_site(
	access_token: String,
	owner_name: String,
	repo_name: String,
	build_command: String,
	_publish_dir: String,
	node_version: String,
	user_agent: String,
) -> Result<(), Error> {
	let octocrab = Octocrab::builder()
		.personal_token(access_token.clone())
		.build()?;
	let client = reqwest::Client::new();

	let response = client
		.get(format!(
			"https://api.github.com/repos/{}/{}/contents/.github/workflows/build.yaml",
			owner_name, repo_name
		))
		.header(AUTHORIZATION, format!("token {}", access_token))
		.header(USER_AGENT, user_agent)
		.send()
		.await?;

	match response.status() {
		reqwest::StatusCode::NOT_FOUND => {
			octocrab
				.repos(&owner_name, &repo_name)
				.create_file(
					".github/workflows/build.yaml",
					"created: build.yaml",
					format!(
						// Change the ubuntu-latest to specifc version later
						// when we support other versions and frameworks
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
				.branch("master")
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
			Ok(())
		}
		reqwest::StatusCode::OK => {
			let body = response.json::<GithubResponseBody>().await?;
			let sha = body.sha;
			println!("all ok already exists");
			println!("sha - {}", sha);
			octocrab
				.repos(&owner_name, &repo_name)
				.update_file(
					".github/workflows/build.yaml",
					"updated: build.yaml",
					format!(
						// Change the ubuntu-latest to specifc version later
						// when we support other versions and frameworks
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
					sha,
				)
				.branch("master")
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
			Ok(())
		}
		_ => Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string()),
	}
}

pub async fn github_actions_for_vanilla_static_site(
	access_token: String,
	owner_name: String,
	repo_name: String,
	user_agent: String,
) -> Result<(), Error> {
	let client = reqwest::Client::new();

	let response = client
		.get(format!("https://api.github.com/repos/{}/{}/contents/.github/workflow/build.yaml", owner_name, repo_name))
		.header("AUTHORIZATION", format!("token {}", access_token))
		.header(USER_AGENT, user_agent)
		.send()
		.await?;

	let octocrab = Octocrab::builder().personal_token(access_token).build()?;
	if response.status() == 404 {
		octocrab
			.repos(&owner_name, &repo_name)
			.create_file(
				".github/workflows/build.yaml",
				"created: build.yaml",
				format!(
					// Change the ubuntu-latest to specifc version later when
					// we support other versions and frameworks
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
		return Ok(());
	} else if response.status() == 200 {
		let body = response.json::<GithubResponseBody>().await?;
		let sha = body.sha;
		octocrab
			.repos(&owner_name, &repo_name)
			.update_file(
				".github/workflows/build.yaml",
				"created: build.yaml",
				format!(
					// Change the ubuntu-latest to specifc version later when
					// we support other versions and frameworks
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
				sha,
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
		return Ok(());
	}
	Error::as_result()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())
}

pub async fn github_actions_for_node_deployment(
	access_token: String,
	owner_name: String,
	repo_name: String,
	build_command: String,
	_publish_dir: String,
	node_version: String,
	user_agent: String,
) -> Result<(), Error> {
	let octocrab = Octocrab::builder()
		.personal_token(access_token.clone())
		.build()?;

	let client = reqwest::Client::new();

	let response = client
		.get(format!("https://api.github.com/repos/{}/{}/contents/.github/workflow/build.yaml", owner_name, repo_name))
		.header("AUTHORIZATION", format!("token {}", access_token))
		.header(USER_AGENT, user_agent)
		.send()
		.await?;

	if response.status() == 404 {
		octocrab
			.repos(owner_name, repo_name)
			.create_file(
				".github/workflows/build.yaml",
				"created: build.yaml",
				format!(
					// Change the ubuntu-latest to specifc version later when
					// we support other versions and frameworks
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
			.branch("master")
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
		return Ok(());
	} else if response.status() == 200 {
		let body = response.json::<GithubResponseBody>().await?;
		let sha = body.sha;
		octocrab
			.repos(owner_name, repo_name)
			.update_file(
				".github/workflows/build.yaml",
				"updated: build.yaml",
				format!(
					// Change the ubuntu-latest to specifc version later when
					// we support other versions and frameworks
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
				sha,
			)
			.branch("master")
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
		return Ok(());
	}
	Error::as_result()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())
}
