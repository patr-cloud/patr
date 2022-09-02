use std::{
	collections::{BTreeMap, HashMap, HashSet},
	fmt::Display,
};

use api_models::{
	models::{
		ci::file_format::{
			CiFlow,
			Decision,
			EnvVarValue,
			Event,
			LabelName,
			Service,
			Step,
			When,
			Work,
		},
		workspace::ci::git_provider::{
			BuildStepStatus,
			GitProviderType,
			RepoStatus,
		},
	},
	utils::Uuid,
};
use eve_rs::AsError;
use globset::{Glob, GlobSetBuilder};

use crate::{
	db::{self, GitProvider},
	models::ci::{Commit, EventType, PullRequest, Tag},
	rabbitmq::{BuildId, BuildStep, BuildStepId},
	service,
	utils::{settings::Settings, Error},
	Database,
};

mod github;

pub use self::github::*;

pub struct Netrc {
	pub machine: String,
	pub login: String,
	pub password: String,
}

impl Display for Netrc {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"machine {} login {} password {}",
			self.machine, self.login, self.password
		)
	}
}

pub enum ParseStatus {
	Success(CiFlow),
	Error,
}

pub fn get_webhook_url_for_repo(
	frontend_domain: &str,
	repo_id: &Uuid,
) -> String {
	format!("{frontend_domain}/webhook/ci/repo/{repo_id}")
}

pub async fn parse_ci_file_content(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	ci_file_content: &[u8],
	request_id: &Uuid,
) -> Result<ParseStatus, Error> {
	let mut ci_flow = match serde_yaml::from_slice::<CiFlow>(ci_file_content) {
		Ok(ci_flow) => ci_flow,
		Err(err) => {
			log::info!("request_id: {request_id} - Error while parsing CI config file {err}");
			return Ok(ParseStatus::Error);
		}
	};

	// check for name duplication
	let CiFlow::Pipeline(pipeline) = &ci_flow;

	let mut step_names = HashSet::new();
	for step in &pipeline.steps {
		let name = match step {
			Step::Work(work) => &work.name,
			Step::Decision(decision) => &decision.name,
		};
		if !step_names.insert(name.as_str()) {
			log::info!(
				"request_id: {} - Duplicate step name `{}` found",
				request_id,
				name
			);
			return Ok(ParseStatus::Error);
		}
	}
	let mut service_names = HashSet::new();
	for service in &pipeline.services {
		if !service_names.insert(service.name.as_str()) {
			log::info!(
				"request_id: {} - Duplicate service name `{}` found",
				request_id,
				service.name
			);
			return Ok(ParseStatus::Error);
		}
	}

	// find and replace secret names with vault secret id
	let workspace_secrets =
		db::get_all_secrets_in_workspace(connection, workspace_id)
			.await?
			.into_iter()
			.map(|secret| (secret.name, secret.id))
			.collect::<HashMap<_, _>>();

	let CiFlow::Pipeline(pipeline) = &mut ci_flow;
	for service in &mut pipeline.services {
		for value in service.env.values_mut() {
			if let EnvVarValue::ValueFromSecret { from_secret } = value {
				if let Some(secret_id) = workspace_secrets.get(&*from_secret) {
					*from_secret = secret_id.to_string();
				} else {
					log::info!(
						"request_id: {} - Invalid secret name `{}` found",
						request_id,
						from_secret
					);
					return Ok(ParseStatus::Error);
				}
			}
		}
	}
	for step in &mut pipeline.steps {
		if let Step::Work(work) = step {
			for value in work.env.values_mut() {
				if let EnvVarValue::ValueFromSecret { from_secret } = value {
					if let Some(secret_id) =
						workspace_secrets.get(&*from_secret)
					{
						*from_secret = secret_id.to_string();
					} else {
						log::info!(
							"request_id: {} - Invalid secret name `{}` found",
							request_id,
							from_secret
						);
						return Ok(ParseStatus::Error);
					}
				}
			}
		}
	}

	Ok(ParseStatus::Success(ci_flow))
}

pub fn evaluate_work_steps_for_ci(
	steps: Vec<Step>,
	event_type: &EventType,
) -> Result<Vec<Work>, Error> {
	let (branch_name, event_type) = match event_type {
		EventType::Commit(commit) => {
			(Some(&commit.committed_branch_name), Event::Commit)
		}
		EventType::Tag(_) => (None, Event::Tag),
		EventType::PullRequest(pull_request) => {
			(Some(&pull_request.to_be_committed_branch_name), Event::Pull)
		}
	};

	let mut works = vec![];

	let mut next_step: Option<LabelName> = None;
	for step in steps {
		if let Some(next_step) = next_step.as_ref() {
			let step_name = match &step {
				Step::Work(work) => &work.name,
				Step::Decision(decision) => &decision.name,
			};

			if next_step != step_name {
				continue;
			}
		}

		match step {
			Step::Work(work) => {
				works.push(work);
				next_step.take();
			}
			Step::Decision(Decision {
				name: _,
				when: When {
					branch: branches,
					event: events,
				},
				then,
				else_,
			}) => {
				let is_branch_matched = {
					if branches.is_empty() {
						true
					} else if let Some(branch_name) = branch_name {
						let globset = {
							let mut globset = GlobSetBuilder::new();
							for glob_str in branches {
								globset.add(Glob::new(&glob_str)?);
							}
							globset.build()?
						};

						globset.is_match(branch_name)
					} else {
						false
					}
				};

				let is_event_matched = if events.is_empty() {
					true
				} else {
					events.iter().any(|event| event == &event_type)
				};

				if is_branch_matched && is_event_matched {
					next_step.replace(then);
				} else if let Some(else_) = else_ {
					next_step.replace(else_);
				} else {
					next_step.take();
				}
			}
		}
	}

	if next_step.is_some() {
		Error::as_result()
			.status(400)
			.body("incomplete workflow in pipeline")?;
	}

	Ok(works)
}

pub async fn add_build_steps_in_db(
	connection: &mut <Database as sqlx::Database>::Connection,
	repo_id: &Uuid,
	build_num: i64,
	works: &[Work],
	request_id: &Uuid,
) -> Result<(), eve_rs::Error<()>> {
	log::trace!("request_id: {request_id} - Adding build steps in db");

	// add cloning as a step
	db::add_ci_step_for_build(
		connection,
		repo_id,
		build_num,
		0,
		"git-clone",
		"",
		"",
		BuildStepStatus::WaitingToStart,
	)
	.await?;

	// add build steps provider in ci file
	for (
		step_count,
		Work {
			name,
			image,
			command,
			env: _,
		},
	) in works.iter().enumerate()
	{
		db::add_ci_step_for_build(
			connection,
			repo_id,
			build_num,
			step_count as i32 + 1,
			name,
			image,
			&Vec::from(command.clone()).join("\n"),
			BuildStepStatus::WaitingToStart,
		)
		.await?;
	}

	Ok(())
}

pub async fn add_build_steps_in_k8s(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
	repo_id: &Uuid,
	repo_name: &str,
	build_id: &BuildId,
	services: Vec<Service>,
	work_steps: Vec<Work>,
	netrc: Option<Netrc>,
	event_type: EventType,
	clone_url: &str,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!("request_id: {request_id} - Adding build steps in k8s");

	let build_machine_type =
		db::get_build_machine_type_for_repo(&mut *connection, repo_id)
			.await?
			.status(500)?;

	service::infrastructure::create_kubernetes_namespace(
		&build_id.get_build_namespace(),
		config,
		request_id,
	)
	.await?;

	service::infrastructure::create_pvc_for_workspace(
		&build_id.get_build_namespace(),
		&build_id.get_pvc_name(),
		build_machine_type.volume as u32,
		config,
		request_id,
	)
	.await?;

	for service in services {
		service::create_background_service_for_ci_in_kubernetes(
			&build_id.get_build_namespace(),
			build_id.repo_workspace_id.as_str(),
			service,
			config,
			request_id,
		)
		.await?;
	}

	// queue clone job
	let git_clone_commands = [
		format!(
			r#"echo "{}" > ~/.netrc"#,
			netrc.map_or("".to_string(), |netrc| netrc.to_string())
		),
		r#"cd "/mnt/workdir/""#.to_string(),
		"set -x".to_string(),
		format!("mkdir {repo_name}"),
		format!(r#"cd "/mnt/workdir/{repo_name}""#),
		r#"export GIT_AUTHOR_NAME=patr-ci"#.to_string(),
		r#"export GIT_AUTHOR_EMAIL=patr-ci@localhost"#.to_string(),
		r#"export GIT_COMMITTER_NAME=patr-ci"#.to_string(),
		r#"export GIT_COMMITTER_EMAIL=patr-ci@localhost"#.to_string(),
		"git init -q".to_string(),
		format!("git remote add origin {clone_url}"),
	]
	.into_iter()
	.chain(get_clone_command_based_on_event_type(&event_type).into_iter())
	.collect();

	service::queue_create_ci_build_step(
		BuildStep {
			id: BuildStepId {
				build_id: build_id.clone(),
				step_id: 0,
			},
			image: "alpine/git".to_string(),
			env_vars: BTreeMap::new(),
			commands: git_clone_commands,
		},
		config,
		request_id,
	)
	.await?;

	// queue build steps
	for (
		step_id,
		Work {
			name: _,
			image,
			command,
			env,
		},
	) in work_steps.into_iter().enumerate()
	{
		// TODO: use step name as dependent instead of step id
		let step_id = 1 + step_id as i32;

		service::queue_create_ci_build_step(
			BuildStep {
				id: BuildStepId {
					build_id: build_id.clone(),
					step_id,
				},
				image,
				env_vars: env,
				commands: vec![
					format!(r#"cd "/mnt/workdir/{repo_name}""#),
					"set -x".to_owned(),
					Vec::from(command).join("\n"),
				],
			},
			config,
			request_id,
		)
		.await?;
	}

	// queue clean up jobs
	service::queue_clean_ci_build_pipeline(
		build_id.clone(),
		config,
		request_id,
	)
	.await?;

	log::debug!("request_id: {request_id} - Successfully created a ci pipeline for build `{build_id}`");
	Ok(())
}

#[derive(Debug, PartialEq, Eq)]
pub struct MutableRepoValues {
	pub repo_owner: String,
	pub repo_name: String,
	pub repo_clone_url: String,
}

pub async fn sync_repos_for_git_provider(
	connection: &mut <Database as sqlx::Database>::Connection,
	git_provider: &GitProvider,
	request_id: &Uuid,
) -> Result<(), Error> {
	match git_provider.git_provider_type {
		GitProviderType::Github => {
			if let Some(access_token) = git_provider.password.clone() {
				sync_github_repos(
					connection,
					&git_provider.id,
					access_token,
					request_id,
				)
				.await?
			}
		}
	}

	Ok(())
}

pub async fn sync_repos_in_db(
	connection: &mut <Database as sqlx::Database>::Connection,
	git_provider_id: &Uuid,
	repos_in_git_provider: HashMap<String, MutableRepoValues>,
	mut repos_in_db: HashMap<String, MutableRepoValues>,
) -> Result<(), sqlx::Error> {
	for (g_repo_id, g_values) in repos_in_git_provider {
		if let Some(db_values) = repos_in_db.remove(&g_repo_id) {
			if g_values != db_values {
				// values differing in db and git-provider, update it now
				db::update_repo_details_for_git_provider(
					connection,
					git_provider_id,
					&g_repo_id,
					&g_values.repo_owner,
					&g_values.repo_name,
					&g_values.repo_clone_url,
				)
				.await?;
			}
		} else {
			// new repo found in git-provider, create it
			db::add_repo_for_git_provider(
				connection,
				git_provider_id,
				&g_repo_id,
				&g_values.repo_owner,
				&g_values.repo_name,
				&g_values.repo_clone_url,
			)
			.await?;
		}
	}

	// missing repos from git-provider, mark as deleted
	for (repo_uid, _) in repos_in_db {
		db::update_repo_status(
			connection,
			git_provider_id,
			&repo_uid,
			RepoStatus::Deleted,
		)
		.await?;
	}

	Ok(())
}

fn get_clone_command_based_on_event_type(
	event_type: &EventType,
) -> Vec<String> {
	match event_type {
		EventType::Commit(Commit {
			repo_owner: _,
			repo_name: _,
			commit_sha,
			committed_branch_name,
		}) => vec![
			format!("git fetch origin +refs/heads/{committed_branch_name}:"),
			format!("git checkout {commit_sha} -b {committed_branch_name}"),
		],
		EventType::Tag(Tag {
			repo_owner: _,
			repo_name: _,
			commit_sha: _,
			tag_name,
		}) => vec![
			format!("git fetch origin +refs/tags/{tag_name}:"),
			format!("git checkout -qf FETCH_HEAD"),
		],
		EventType::PullRequest(PullRequest {
			head_repo_owner: _,
			head_repo_name: _,
			commit_sha,
			pr_number: pull_number,
			to_be_committed_branch_name,
		}) => vec![
			format!(
				"git fetch origin +refs/heads/{to_be_committed_branch_name}:"
			),
			format!("git checkout {to_be_committed_branch_name}"),
			format!("git fetch origin +refs/pull/{pull_number}/head:"),
			format!("git merge {commit_sha}"),
		],
	}
}
