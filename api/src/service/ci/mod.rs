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
			Service,
			Step,
			When,
			Work,
		},
		workspace::{
			ci::git_provider::{BuildStepStatus, GitProviderType, RepoStatus},
			region::RegionStatus,
		},
	},
	utils::Uuid,
};
use chrono::Utc;
use eve_rs::AsError;
use globset::{Glob, GlobSetBuilder};
use sqlx::types::Json;

use crate::{
	db::{self, GitProvider},
	models::{
		ci::{Commit, EventType, PullRequest, Tag},
		rbac,
	},
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
	Error(String),
}

pub fn get_webhook_url_for_repo(api_url: &str, repo_id: &Uuid) -> String {
	format!("{api_url}/webhook/ci/repo/{repo_id}")
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
			return Ok(ParseStatus::Error(String::from(
				"Error: CI file parse error",
			)));
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
			return Ok(ParseStatus::Error(format!(
				"Error: Duplicate step name found - {}",
				name
			)));
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
			return Ok(ParseStatus::Error(format!(
				"Error: Duplicate service name found - {}",
				service.name
			)));
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
		for value in service.environment.values_mut() {
			if let EnvVarValue::ValueFromSecret { from_secret } = value {
				if let Some(secret_id) = workspace_secrets.get(&*from_secret) {
					*from_secret = secret_id.to_string();
				} else {
					log::info!(
						"request_id: {} - Invalid secret name `{}` found",
						request_id,
						from_secret
					);
					return Ok(ParseStatus::Error(format!(
						"Error: Invalid secret name - {}",
						from_secret
					)));
				}
			}
		}
	}
	for step in &mut pipeline.steps {
		if let Step::Work(work) = step {
			for value in work.environment.values_mut() {
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
						return Ok(ParseStatus::Error(format!(
							"Error: Invalid secret name - {}",
							from_secret
						)));
					}
				}
			}
		}
	}

	Ok(ParseStatus::Success(ci_flow))
}

pub enum EvaluationStatus {
	Success(Vec<Work>),
	Error(String),
}

pub fn evaluate_work_steps_for_ci(
	steps: Vec<Step>,
	event_type: &EventType,
) -> Result<EvaluationStatus, Error> {
	let (branch_name, event_type) = match event_type {
		EventType::Commit(commit) => {
			(Some(&commit.committed_branch_name), Event::Commit)
		}
		EventType::Tag(_) => (None, Event::Tag),
		EventType::PullRequest(pull_request) => {
			(Some(&pull_request.to_be_committed_branch_name), Event::Pull)
		}
	};

	// if there are no decision blocks and no next step defined,
	// do the work in the defined order
	let is_steps_linear = steps
		.iter()
		.all(|step| matches!(step, Step::Work(work) if work.next.is_none()));
	if is_steps_linear {
		let works = steps
			.into_iter()
			.filter_map(|step| match step {
				Step::Work(work) => Some(work),
				_ => None, // safe to do as it is validated in previous check
			})
			.collect();
		return Ok(EvaluationStatus::Success(works));
	};

	// handle graph of steps
	let first_step = {
		let mut all_step_names = steps
			.iter()
			.map(|step| match step {
				Step::Work(work) => &work.name,
				Step::Decision(decision) => &decision.name,
			})
			.collect::<HashSet<_>>();

		steps
			.iter()
			.flat_map(|step| match step {
				Step::Work(work) => vec![work.next.clone()].into_iter(),
				Step::Decision(decision) => {
					vec![Some(decision.then.clone()), decision.else_.clone()]
						.into_iter()
				}
			})
			.flatten()
			.for_each(|label| {
				all_step_names.remove(&label);
			});

		if all_step_names.len() == 1 {
			all_step_names.into_iter().next().unwrap().clone()
		} else {
			return Ok(EvaluationStatus::Error(
				"Unable to find starting step in ci".to_owned(),
			));
		}
	};

	// from first step find until next step in none
	let mut works = vec![];
	let mut steps = steps
		.into_iter()
		.map(|step| {
			let name = match &step {
				Step::Work(work) => work.name.clone(),
				Step::Decision(decision) => decision.name.clone(),
			};
			(name, step)
		})
		.collect::<HashMap<_, _>>();

	let mut next_step = Some(first_step);
	while let Some(next_step_name) = next_step.take() {
		let step = if let Some(step) = steps.remove(&next_step_name) {
			step
		} else {
			return Ok(EvaluationStatus::Error(format!(
				"Error: unknown step name {next_step_name}"
			)));
		};

		match step {
			Step::Work(work) => {
				next_step = work.next.clone();
				works.push(work);
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
					next_step = Some(then);
				} else {
					next_step = else_
				}
			}
		};
	}

	Ok(EvaluationStatus::Success(works))
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
			commands: command,
			environment: _,
			next: _,
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
		db::get_all_default_regions(&mut *connection)
			.await?
			.into_iter()
			.find_map(|region| {
				if region.status == RegionStatus::Active {
					region.config_file.map(|Json(config)| config)
				} else {
					None
				}
			})
			.status(500)?,
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
		// "set -x".to_string(),
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
			commands: command,
			environment: env,
			next: _,
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
					// "set -x".to_owned(),
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
					&git_provider.workspace_id,
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
	workspace_id: &Uuid,
	git_provider_id: &Uuid,
	repos_in_git_provider: HashMap<String, MutableRepoValues>,
	mut repos_in_db: HashMap<String, MutableRepoValues>,
	reqeust_id: &Uuid,
) -> Result<(), Error> {
	for (gp_repo_id, gp_values) in repos_in_git_provider {
		if let Some(db_values) = repos_in_db.remove(&gp_repo_id) {
			if gp_values != db_values {
				// values differing in db and git-provider, update it now
				db::update_repo_details_for_git_provider(
					connection,
					git_provider_id,
					&gp_repo_id,
					&gp_values.repo_owner,
					&gp_values.repo_name,
					&gp_values.repo_clone_url,
				)
				.await?;
			}
		} else {
			// new repo found in git-provider, create it
			service::add_repo_for_git_provider(
				connection,
				git_provider_id,
				&gp_repo_id,
				&gp_values.repo_owner,
				&gp_values.repo_name,
				&gp_values.repo_clone_url,
				workspace_id,
				reqeust_id,
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

pub async fn add_repo_for_git_provider(
	connection: &mut <Database as sqlx::Database>::Connection,
	git_provider_id: &Uuid,
	git_provider_repo_uid: &str,
	repo_owner: &str,
	repo_name: &str,
	clone_url: &str,
	workspace_id: &Uuid,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!("request_id: {} - Creating a new repo for CI", request_id);

	// get a unique repo id
	let repo_id = db::generate_new_resource_id(connection).await?;

	// add a resource entry for repo
	let created_time = Utc::now();
	db::create_resource(
		connection,
		&repo_id,
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::CI_REPO)
			.unwrap(),
		workspace_id,
		&created_time,
	)
	.await?;

	db::begin_deferred_constraints(connection).await?;

	db::add_repo_for_git_provider(
		connection,
		&repo_id,
		git_provider_id,
		git_provider_repo_uid,
		repo_owner,
		repo_name,
		clone_url,
	)
	.await?;

	db::end_deferred_constraints(connection).await?;

	Ok(())
}

fn get_clone_command_based_on_event_type(
	event_type: &EventType,
) -> Vec<String> {
	match event_type {
		EventType::Commit(Commit {
			repo_owner: _,
			repo_name: _,
			author: _,
			commit_message: _,
			commit_sha,
			committed_branch_name,
		}) => vec![
			format!("git fetch origin +refs/heads/{committed_branch_name}:"),
			format!("git checkout {commit_sha} -b {committed_branch_name}"),
		],
		EventType::Tag(Tag {
			repo_owner: _,
			repo_name: _,
			author: _,
			commit_message: _,
			commit_sha: _,
			tag_name,
		}) => vec![
			format!("git fetch origin +refs/tags/{tag_name}:"),
			format!("git checkout -qf FETCH_HEAD"),
		],
		EventType::PullRequest(PullRequest {
			pr_repo_owner: _,
			pr_repo_name: _,
			repo_owner: _,
			repo_name: _,
			author: _,
			pr_title: _,
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
