use eve_rs::AsError;
use octocrab::{models::repos::GitUser, Octocrab};
use reqwest::header::{AUTHORIZATION, USER_AGENT};

use crate::{error, models::db_mapping::GithubResponseBody, utils::Error};

// Static sites

pub async fn github_actions_for_node_static_site(
	access_token: String,
	owner_name: String,
	repo_name: String,
	build_command: String,
	_publish_dir: String,
	version: String,
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
          node-version: {version}

        steps:
        - uses: actions/checkout@v3
        - name: using node ${{matrix.node-version}}
            uses: actions/setup-node@v2
            with: 
            node-version: ${{matrix.node-version}}
            cache: 'npm'
        - run: npm install
        - run: npm run test --if-present
        - run: {build_command}

        - name: Zip build folder
          run: <todo>
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
			Ok(())
		}
		reqwest::StatusCode::OK => {
			let body = response.json::<GithubResponseBody>().await?;
			let sha = body.sha;
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
      branches: [main]

jobs:
    build:

        runs-on: ubuntu-latest

        strategy:
        matrix: 
          node-version: {version}

        steps:
        - uses: actions/checkout@v3
        - name: using node ${{matrix.node-version}}
        	uses: actions/setup-node@v2
        	with: 
        	node-version: ${{matrix.node-version}}
        	cache: 'npm'
        - run: npm install
        - run: npm run test --if-present
        - run: {build_command}
        
        - name: Zip build folder
          run: <todo>

        - name: Ship
		  run: <ship zip folder>
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
					// Change the ubuntu-latest to specifc version later
					r#"
name: Github action for your static site

on:
    push:
    branch: [main]

jobs:
    build:
    runs-on: ubuntu-latest
    steps:
	- uses: actions/checkout@v3
	- name: Archive Release
        uses: {owner_name}/{repo_name}@master
        with:
        type: 'zip'
        filename: 'release.zip'
	- name: push to patr
    - run: echo TODO
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
					// Change the ubuntu-latest to specifc version later
					r#"
name: Github action for your static site

on:
    push:
    branch: [main]

jobs:
    build:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    - name: Archive Release
        uses: {owner_name}/{repo_name}@master
        with:
        type: 'zip'
        filename: 'release.zip'
    - name: push to patr
    - run: echo TODO
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

// Deployments

pub async fn github_actions_for_node_deployment(
	access_token: String,
	owner_name: String,
	repo_name: &str,
	_build_command: String,
	_publish_dir: String,
	version: String,
	user_agent: String,
	username: String,
	tag: &str,
	workspace_name: &str,
	docker_repo_name: &str,
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
					// Change the ubuntu-latest to specifc version later
					r#"
name: Github action for your deployment
on:
    push:
      branches: main

jobs:
    build:

        runs-on: ubuntu-latest
        strategy:
            matrix: 
                node-version: [{version}]
        steps:
        - uses: actions/checkout@v3
        - name: Using node {version}
          uses: actions/setup-node@v2
          with: 
            node-version: {version}
        - run: npm install
        - name: Creating a Dockerfile
          run: |
            echo "
            FROM node:{version}
            WORKDIR /app
            COPY . .
            RUN npm install
            CMD ["node", "server"]
            " > Dockerfile

        - name: Docker login
          uses: docker/login-action@v1 
          with:
            registry: registry.patr.cloud
            username: {username}
            password: ${{{{ secrets.REGISTRY_PASSWORD }}}}

        - name: Build image from Dockerfile and push to patr-registry
          run: |
            docker build . -t {username}/deployment
            docker tag {username}/deployment registry.patr.cloud/{workspace_name}/{docker_repo_name}:{tag} 
            docker push registry.patr.cloud/{workspace_name}/{docker_repo_name}:{tag} 
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
		println!("{:#?}", body);
		let sha = body.sha;
		octocrab
			.repos(owner_name, repo_name)
			.update_file(
				".github/workflows/build.yaml",
				"updated: build.yaml",
				format!(
					// Change the ubuntu-latest to specifc version later
					r#"
name: Github action for your deployment
on:
    push:
        branches: main

jobs:
    build:

         runs-on: ubuntu-latest
         strategy:
            matrix: 
                node-version: [{version}]
         steps:
         - uses: actions/checkout@v3
         - name: Using node {version}
           uses: actions/setup-node@v2
           with: 
            node-version: {version}
         - run: npm install
         - name: Creating a Dockerfile
           run: |
            echo "
            FROM node:{version}
            WORKDIR /app
            COPY . .
            RUN npm install
            CMD ["node", "server"]
            " > Dockerfile

         - name: Docker login
           uses: docker/login-action@v1 
           with:
            registry: registry.patr.cloud
            username: {username}
            password: ${{{{ secrets.REGISTRY_PASSWORD }}}}

         - name: Build image from Dockerfile and push to patr-registry
           run: |
            docker build . -t {username}/deployment
            docker tag {username}/deployment registry.patr.cloud/{workspace_name}/{docker_repo_name}:{tag} 
            docker push registry.patr.cloud/{workspace_name}/{docker_repo_name}:{tag} 
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

pub async fn github_actions_for_django_deployment(
	access_token: String,
	owner_name: String,
	repo_name: &str,
	_build_command: String,
	_publish_dir: String,
	version: String,
	user_agent: String,
	username: String,
	tag: &str,
	workspace_name: &str,
	docker_repo_name: &str,
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
					// Change the ubuntu-latest to specifc version later
					r#"
name: Github action for your deployment
on:
    push:
      branches: [main]

jobs:
    build:
        runs-on: ubuntu-latest

        steps:
        - uses: actions/checkout@v3
        - name: Set up Python {version}
          uses: actions/setup-python@v3
          with:
            python-version: {version}
        - name: Install Dependencies
          run: |
            python -m pip install --upgrade pip
            if [ -f requirements.txt ]; then
            	pip install -r requirements.txt
            else 
            	pip freeze > requirements.txt
            fi
        - name: Creting a Dockerfile
          run: |
            echo "
              FROM python:{version}
              WORKDIR /app
              COPY . .
              RUN pip3 install -r requirements.txt
              CMD ["python3", "manage.py", "runserver"]
              " > Dockerfile
        - name: Docker login
          uses: docker/login-action@v1 
          with:
            registry: registry.patr.cloud
            username: {username}
            password: ${{{{ secrets.REGISTRY_PASSWORD }}}}
        - name: Build image from Dockerfile and push to patr-registry
          run: |
            docker build . -t {username}/deployment
            docker tag {username}/deployment registry.patr.cloud/{workspace_name}/{docker_repo_name}:{tag} 
            docker push registry.patr.cloud/{workspace_name}/{docker_repo_name}:{tag}
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
			.repos(owner_name, repo_name)
			.update_file(
				".github/workflows/build.yaml",
				"updated: build.yaml",
				format!(
					// Change the ubuntu-latest to specifc version later
					r#"
name: Github action for your deployment
on:
    push:
        branches: [main]

jobs:
    build:
        runs-on: ubuntu-latest

        steps:
        - uses: actions/checkout@v3
        - name: Set up Python {version}
          uses: actions/setup-python@v3
          with:
            python-version: {version}
        - name: Install Dependencies
          run: |
            python -m pip install --upgrade pip
            if [ -f requirements.txt ]; then
            	pip install -r requirements.txt
            else 
            	pip freeze > requirements.txt
            fi
        - name: Creting a Dockerfile
          run: |
            echo "
              FROM python:{version}
              WORKDIR /app
              COPY . .
              RUN pip3 install -r requirements.txt
              CMD ["python3", "manage.py", "runserver"]
              " > Dockerfile

        - name: Docker login
          uses: docker/login-action@v1 
          with:
            registry: registry.patr.cloud
            username: {username}
            password: ${{{{ secrets.REGISTRY_PASSWORD }}}}

        - name: Build image from Dockerfile and push to patr-registry
          run: |
            docker build . -t {username}/deployment
            docker tag {username}/deployment registry.patr.cloud/{workspace_name}/{docker_repo_name}:{tag} 
            docker push registry.patr.cloud/{workspace_name}/{docker_repo_name}:{tag}
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

pub async fn github_actions_for_flask_deployment(
	access_token: String,
	owner_name: String,
	repo_name: &str,
	_build_command: String,
	_publish_dir: String,
	version: String,
	user_agent: String,
	username: String,
	tag: &str,
	workspace_name: &str,
	docker_repo_name: &str,
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
					// Change the ubuntu-latest to specifc version later
					r#"
name: Github action for your deployment
on:
    push:
      branches: [main]

jobs:
    build:
        runs-on: ubuntu-latest

        steps:
        - uses: actions/checkout@v3
        - name: Set up Python {version}
          uses: actions/setup-python@v3
          with:
            python-version: {version}
        - name: Install Dependencies
          run: |
            python -m pip install --upgrade pip
            if [ -f requirements.txt ]; then
                pip install -r requirements.txt
            else 
                pip freeze > requirements.txt
            fi
        - name: Creting a Dockerfile
          run: |
            echo "
              FROM python:{version}
              WORKDIR /app
              COPY . .
              RUN pip3 install -r requirements.txt
              CMD ["python3", "src/app.py"]
              " > Dockerfile

        - name: Docker login
          uses: docker/login-action@v1 
          with:
            registry: registry.patr.cloud
            username: {username}
            password: ${{{{ secrets.REGISTRY_PASSWORD }}}}

        - name: Build image from Dockerfile and push to patr-registry
          run: |
            docker build . -t {username}/deployment
            docker tag {username}/deployment registry.patr.cloud/{workspace_name}/{docker_repo_name}:{tag} 
            docker push registry.patr.cloud/{workspace_name}/{docker_repo_name}:{tag}
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
			.repos(owner_name, repo_name)
			.update_file(
				".github/workflows/build.yaml",
				"updated: build.yaml",
				format!(
					// Change the ubuntu-latest to specifc version later
					r#"
name: Github action for your deployment
on:
    push:
      branches: [main]

jobs:
    build:

        runs-on: ubuntu-latest

        steps:
        - uses: actions/checkout@v3
        - name: Set up Python {version}
          uses: actions/setup-python@v3
          with:
            python-version: {version}
        - name: Install Dependencies
          run: |
            python -m pip install --upgrade pip
            if [ -f requirements.txt ]; then
                pip install -r requirements.txt
            else 
                pip freeze > requirements.txt
            fi
        - name: Creting a Dockerfile
          run: |
            echo "
              FROM python:{version}
              WORKDIR /app
              COPY . .
              RUN pip3 install -r requirements.txt
              CMD ["python3", "src/app.py"]
              " > Dockerfile

        - name: Docker login
          uses: docker/login-action@v1 
          with:
            registry: registry.patr.cloud
            username: {username}
            password: ${{{{ secrets.REGISTRY_PASSWORD }}}}

        - name: Build image from Dockerfile and push to patr-registry
          run: |
            docker build . -t {username}/deployment
            docker tag {username}/deployment registry.patr.cloud/{workspace_name}/{docker_repo_name}:{tag} 
            docker push registry.patr.cloud/{workspace_name}/{docker_repo_name}:{tag}
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

pub async fn github_actions_for_spring_deployment(
	access_token: String,
	owner_name: String,
	repo_name: &str,
	_build_command: String,
	_publish_dir: String,
	version: String,
	user_agent: String,
	username: String,
	tag: &str,
	workspace_name: &str,
	docker_repo_name: &str,
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
					// Change the ubuntu-latest to specifc version later
					r#"
name: Github action for your deployment

on:
    push:
      branches: [main]

jobs:
    build:

        runs-on: ubuntu-latest

        steps:
        - uses: actions/checkout@v3
        - name: Set up JDK 11
          uses: actions/setup-java@v3
          with:
            java-version: {version}
            distribution: 'temurin'
            cache: maven
        - name: Build with Maven
        - run: mvn clean install
        - run: mvn package

        - name: Create a Dockerfile
        - run: echo 
"
FROM openjdk
WORKDIR .
ADD target/<build-name>
ENTRYPOINT ["java", "-jar", "<build-name>"]
" > Dockerfile

        - name: Build image from Dockerfile and push to patr-registry
        - run: |
            docker login -u {username} -p ${{REGISTRY_PASSWORD}} registry.patr.cloud
            docker build . -t {username}/deployment
            docker tag {username}/deployment registry.patr.cloud/{workspace_name}/{docker_repo_name}:{tag} 
            docker push registry.patr.cloud/{workspace_name}/{docker_repo_name}:{tag}
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
			.repos(owner_name, repo_name)
			.update_file(
				".github/workflows/build.yaml",
				"updated: build.yaml",
				format!(
					// Change the ubuntu-latest to specifc version later
					r#"
name: Github action for your deployment

on:
    push:
    branch: [main]

jobs:
    build:
    
        runs-on: ubuntu-latest
    
        steps:
        - uses: actions/checkout@v3
        - name: Set up JDK 11
        uses: actions/setup-java@v3
        with:
            java-version: {version}
            distribution: 'temurin'
            cache: maven
        - name: Build with Maven
          run: mvn clean install
          run: mvn package
        - name: Create a Dockerfile
          run: echo 
"
FROM openjdk
WORKDIR .
ADD target/<build-name>
ENTRYPOINT ["java", "-jar", "<build-name>"]
" > Dockerfile

        - name: Build image from Dockerfile and push to patr-registry
          run: |
            docker login -u {username} -p ${{REGISTRY_PASSWORD}} registry.patr.cloud
            docker build . -t {username}/deployment
            docker tag {username}/deployment registry.patr.cloud/{workspace_name}/{docker_repo_name}:{tag} 
            docker push registry.patr.cloud/{workspace_name}/{docker_repo_name}:{tag}
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

pub async fn github_actions_for_rust_deployment(
	access_token: String,
	owner_name: String,
	repo_name: &str,
	_build_command: String,
	_publish_dir: String,
	_version: String,
	user_agent: String,
	username: String,
	tag: &str,
	workspace_name: &str,
	docker_repo_name: &str,
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
					// Change the ubuntu-latest to specifc version later
					r#"
name: Github action for your deployment

on:
    push:
    branches: [main]

jobs:
    build:
        runs-on: ubuntu-latest

        steps:
        - name: Checkout code
          uses: actions/checkout@v3
        - name: Test and build
          run: |
            cargo test
            cargo build

        - name: Create Dockerfile
        - run: echo 
"
<todo>
" > Dockerfile

        - name: Build image from Dockerfile and push to patr-registry
        - run: |
            docker login -u {username} -p ${{REGISTRY_PASSWORD}} registry.patr.cloud
            docker build . -t {username}/deployment
            docker tag {username}/deployment registry.patr.cloud/{workspace_name}/{docker_repo_name}:{tag} 
            docker push registry.patr.cloud/{workspace_name}/{docker_repo_name}:{tag}
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
			.repos(owner_name, repo_name)
			.update_file(
				".github/workflows/build.yaml",
				"updated: build.yaml",
				format!(
					// Change the ubuntu-latest to specifc version later
					r#"
name: Github action for your deployment

on:
    push:
    branches: [main]

jobs:
    build:
        runs-on: ubuntu-latest

        steps:
        - name: Checkout code
          uses: actions/checkout@v3
        - name: Test and build
          run: |
            cargo test
            cargo build

        - name: Create Dockerfile
          run: echo 
"
<todo>
" > Dockerfile

        - name: Build image from Dockerfile and push to patr-registry
        - run: |
            docker login -u {username} -p ${{REGISTRY_PASSWORD}} registry.patr.cloud
            docker build . -t {username}/deployment
            docker tag {username}/deployment registry.patr.cloud/{workspace_name}/{docker_repo_name}:{tag} 
            docker push registry.patr.cloud/{workspace_name}/{docker_repo_name}:{tag}
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
