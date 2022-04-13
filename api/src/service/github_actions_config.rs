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
        node-version: {version}
steps:
- uses: actions/checkout@v3
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
	version: String,
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
            node-version: {version}
	
        steps:
        - uses: actions/checkout@v3
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
            node-version: {version}
	
        steps:
        - uses: actions/checkout@v3
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

pub async fn github_actions_for_django_deployment(
	access_token: String,
	owner_name: String,
	repo_name: String,
	_build_command: String,
	_publish_dir: String,
	version: String,
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
            max-parallel: 4
            matrix:
                python-version: {version}
	
        steps:
        - uses: actions/checkout@v3
        - name: Set up Python ${{ matrix.python-version }}
              uses: actions/setup-python@v3
              with:
                  python-version: ${{ matrix.python-version }}
        - name: Install Dependencies
        run: |
            python -m pip install --upgrade pip
            pip install -r requirements.txt
        - name: Run Tests
          run: |
              python manage.py test

***************************************************************************
- run: echo
"
FROM python3

ENV PYTHONDONTWRITEBYTECODE 1
ENV PYTHONUNBUFFERED 1

WORKDIR .
RUN pip install --upgrade pip
RUN pip freeze > requirements.txt

RUN pip install -r requirements.txt

COPY . .

EXPOSE 8888
CMD ["python3", "manage.py", "runserver", "0.0.0.0:8888"]
" > Dockerfile

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
	        max-parallel: 4
	        matrix:
		        python-version: {version}

        steps:
        - uses: actions/checkout@v3
        - name: Set up Python ${{ matrix.python-version }}
		        uses: actions/setup-python@v3
		        with:
			        python-version: ${{ matrix.python-version }}
        - name: Install Dependencies
        run: |
	        python -m pip install --upgrade pip
	        pip install -r requirements.txt
        - name: Run Tests
	        run: |
		        python manage.py test

***************************************************************************
- run: echo
"
FROM python3

ENV PYTHONDONTWRITEBYTECODE 1
ENV PYTHONUNBUFFERED 1

WORKDIR .
RUN pip install --upgrade pip
RUN pip freeze > requirements.txt

RUN pip install -r requirements.txt

COPY . .

EXPOSE 8888
CMD ["python3", "manage.py", "runserver", "0.0.0.0:8888"]
" > Dockerfile

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

pub async fn github_actions_for_flask_deployment(
	access_token: String,
	owner_name: String,
	repo_name: String,
	_build_command: String,
	_publish_dir: String,
	version: String,
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
            max-parallel: 4
            matrix:
                python-version: {version}
	
        steps:
        - uses: actions/checkout@v3
        - name: Set up Python ${{ matrix.python-version }}
            uses: actions/setup-python@v3
            with:
                python-version: ${{ matrix.python-version }}
        - name: Install Dependencies
        run: |
            python -m pip install --upgrade pip
            pip install -r requirements.txt
            TODO - test command for flask 
**********************************************************************
- run: echo 
"
FROM python3

WORKDIR /app

COPY . .

RUN pip install -r requirements.txt

COPY . .

CMD ["python3", "app.py"]
" > Dockerfile

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
            max-parallel: 4
            matrix:
                python-version: {version}

        steps:
        - uses: actions/checkout@v3
        - name: Set up Python ${{ matrix.python-version }}
            uses: actions/setup-python@v3
            with:
                python-version: ${{ matrix.python-version }}
        - name: Install Dependencies
        run: |
            python -m pip install --upgrade pip
            pip install -r requirements.txt
            TODO - test command for flask 
	**********************************************************************
- run: echo 
"
FROM python3

WORKDIR /app

COPY . .

RUN pip install -r requirements.txt

COPY . .

CMD ["python3", "app.py"]
" > Dockerfile

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

pub async fn github_actions_for_spring_deployment(
	access_token: String,
	owner_name: String,
	repo_name: String,
	_build_command: String,
	_publish_dir: String,
	version: String,
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

    steps:
    - uses: actions/checkout@v3
    - name: Set up JDK 11
    uses: actions/setup-java@v3
    with:
        java-version: {version}
        distribution: 'temurin'
        cache: maven
    name: Build with Maven
    run: mvn clean install
**********************************************************************************************************
- run: echo 
"
FROM python3

WORKDIR /app

COPY . .

RUN pip install -r requirements.txt

COPY . .

CMD ["python3", "app.py"]
" > Dockerfile

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
    
        steps:
        - uses: actions/checkout@v3
        - name: Set up JDK 11
        uses: actions/setup-java@v3
        with:
            java-version: {version}
            distribution: 'temurin'
            cache: maven
        name: Build with Maven
        run: mvn -B package --file pom.xml
**********************************************************************************************************
- run: echo 
"
FROM python3

WORKDIR /app

COPY . .

RUN pip install -r requirements.txt

COPY . .

CMD ["python3", "app.py"]
" > Dockerfile

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

pub async fn github_actions_for_angular_deployment(
	access_token: String,
	owner_name: String,
	repo_name: String,
	_build_command: String,
	_publish_dir: String,
	_version: String,
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

steps:
- uses: actions/checkout@v3
- run: echo 
"
FROM python3

WORKDIR /app

COPY . .

RUN pip install -r requirements.txt

COPY . .

CMD ["python3", "app.py"]
" > Dockerfile

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
	
steps:
- uses: actions/checkout@v3
- run: echo 
"
FROM python3

WORKDIR /app

COPY . .

RUN pip install -r requirements.txt

COPY . .

CMD ["python3", "app.py"]
" > Dockerfile

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

pub async fn github_actions_for_ror_deployment(
	access_token: String,
	owner_name: String,
	repo_name: String,
	_build_command: String,
	_publish_dir: String,
	_version: String,
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

    steps:
    - name: Checkout code
      uses: actions/checkout@v3
    - name: setup ruby environment
      uses: ruby/setup-ruby@8f312efe1262fb463d906e9bf040319394c18d3e # v1.92
      with:
          bundler-cache: true
    - name: Run tests
      run: bin/rake

************************************************************
- run: echo 
"
FROM python3

WORKDIR /app

COPY . .

RUN pip install -r requirements.txt

COPY . .

CMD ["python3", "app.py"]
" > Dockerfile

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

        steps:
        - name: Checkout code
          uses: actions/checkout@v3
        - name: setup ruby environment
          uses: ruby/setup-ruby@8f312efe1262fb463d906e9bf040319394c18d3e # v1.92
          with:
	          bundler-cache: true
        - name: Run tests
          run: bin/rake

************************************************************
-
- run: echo 
"
FROM python3

WORKDIR /app

COPY . .

RUN pip install -r requirements.txt

COPY . .

CMD ["python3", "app.py"]
" > Dockerfile

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
