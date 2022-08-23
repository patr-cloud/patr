use std::{
	collections::{HashMap, HashSet},
	fmt::Display,
};

use api_models::{
	models::workspace::ci2::github::{
		BuildStepStatus,
		GitProviderType,
		RepoStatus,
	},
	utils::Uuid,
};
use eve_rs::AsError;
use k8s_openapi::{
	api::{
		apps::v1::{Deployment, DeploymentSpec},
		core::v1::{
			Container,
			EnvVar,
			Namespace,
			PersistentVolumeClaim,
			PersistentVolumeClaimSpec,
			PodSpec,
			PodTemplateSpec,
			ResourceRequirements,
			Service,
			ServicePort,
			ServiceSpec,
		},
	},
	apimachinery::pkg::{
		api::resource::Quantity,
		apis::meta::v1::LabelSelector,
		util::intstr::IntOrString,
	},
};
use kube::{api::ObjectMeta, Api};

use crate::{
	db::{self, GitProvider},
	models::ci::{
		self,
		file_format::{CiFlow, EnvVarValue, Step},
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
	Error,
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
		if !step_names.insert(step.name.as_str()) {
			log::info!(
				"request_id: {} - Duplicate step name `{}` found",
				request_id,
				step.name
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
		for env in &mut service.env {
			if let EnvVarValue::ValueFromSecret(secret_name) = &mut env.value {
				if let Some(secret_id) = workspace_secrets.get(&*secret_name) {
					*secret_name = secret_id.to_string();
				} else {
					log::info!(
						"request_id: {} - Invalid secret name `{}` found",
						request_id,
						secret_name
					);
					return Ok(ParseStatus::Error);
				}
			}
		}
	}
	for step in &mut pipeline.steps {
		for env in &mut step.env {
			if let EnvVarValue::ValueFromSecret(secret_name) = &mut env.value {
				if let Some(secret_id) = workspace_secrets.get(&*secret_name) {
					*secret_name = secret_id.to_string();
				} else {
					log::info!(
						"request_id: {} - Invalid secret name `{}` found",
						request_id,
						secret_name
					);
					return Ok(ParseStatus::Error);
				}
			}
		}
	}

	Ok(ParseStatus::Success(ci_flow))
}

pub async fn add_build_steps_in_db(
	connection: &mut <Database as sqlx::Database>::Connection,
	repo_id: &Uuid,
	build_num: i64,
	ci_flow: &CiFlow,
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
	let CiFlow::Pipeline(pipeline) = ci_flow;
	for (
		step_count,
		Step {
			name,
			image,
			commands,
			env: _,
		},
	) in pipeline.steps.iter().enumerate()
	{
		db::add_ci_step_for_build(
			connection,
			repo_id,
			build_num,
			step_count as i32 + 1,
			name,
			image,
			&commands.to_string(),
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
	build_id: &BuildId,
	ci_flow: CiFlow,
	netrc: Option<Netrc>,
	repo_clone_url: &str,
	repo_name: &str,
	git_commit: &str,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!("request_id: {request_id} - Adding build steps in k8s");

	let build_machine_type =
		db::get_build_machine_type_for_repo(&mut *connection, repo_id)
			.await?
			.status(500)?;

	let kube_client = service::get_kubernetes_config(config).await?;

	// create a new namespace
	Api::<Namespace>::all(kube_client.clone())
		.create(
			&Default::default(),
			&Namespace {
				metadata: ObjectMeta {
					name: Some(build_id.get_build_namespace()),
					..Default::default()
				},
				..Default::default()
			},
		)
		.await?;

	// create a storage space for building
	Api::<PersistentVolumeClaim>::namespaced(
		kube_client.clone(),
		&build_id.get_build_namespace(),
	)
	.create(
		&Default::default(),
		&PersistentVolumeClaim {
			metadata: ObjectMeta {
				name: Some(build_id.get_pvc_name()),
				..Default::default()
			},
			spec: Some(PersistentVolumeClaimSpec {
				access_modes: Some(vec!["ReadWriteOnce".to_string()]),
				storage_class_name: Some("do-block-storage".to_string()),
				resources: Some(ResourceRequirements {
					requests: Some(
						[(
							"storage".to_string(),
							Quantity(format!(
								"{}Gi",
								build_machine_type.volume
							)),
						)]
						.into(),
					),
					..Default::default()
				}),
				volume_mode: Some("Filesystem".to_string()),
				..Default::default()
			}),
			..Default::default()
		},
	)
	.await?;

	let CiFlow::Pipeline(pipeline) = ci_flow;
	for service in &pipeline.services {
		create_background_service_for_pipeline(
			kube_client.clone(),
			build_id,
			service,
			config,
		)
		.await?;
	}

	// queue clone job
	service::queue_create_ci_build_step(
		BuildStep {
			id: BuildStepId {
				build_id: build_id.clone(),
				step_id: 0,
			},
			image: "alpine/git".to_string(),
			env_vars: vec![],
			commands: vec![
				format!(
					r#"echo "{}" > ~/.netrc"#,
					netrc.map_or("".to_string(), |netrc| netrc.to_string())
				),
				r#"cd "/mnt/workdir/""#.to_string(),
				"set -x".to_string(),
				format!(r#"git clone "{repo_clone_url}""#),
				format!(r#"cd "/mnt/workdir/{repo_name}""#),
				format!(r#"git checkout "{git_commit}""#),
			],
		},
		config,
		request_id,
	)
	.await?;

	// queue build steps
	for (
		step_id,
		Step {
			name: _,
			image,
			commands,
			env,
		},
	) in pipeline.steps.into_iter().enumerate()
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
					commands.to_string(),
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

async fn create_background_service_for_pipeline(
	kube_client: kube::Client,
	build_id: &BuildId,
	service: &ci::file_format::Service,
	config: &Settings,
) -> Result<(), Error> {
	let env = (!service.env.is_empty()).then(|| {
		service
			.env
			.iter()
			.map(|ci::file_format::EnvVar { name, value }| EnvVar {
				name: name.clone(),
				value: Some(match value {
					EnvVarValue::Value(value) => value.clone(),
					EnvVarValue::ValueFromSecret(from_secret) => format!(
						"vault:secret/data/{}/{}#data",
						build_id.repo_workspace_id, from_secret
					),
				}),
				..Default::default()
			})
			.collect()
	});

	let annotations = [
		(
			"vault.security.banzaicloud.io/vault-addr".to_string(),
			config.vault.address.clone(),
		),
		(
			"vault.security.banzaicloud.io/vault-role".to_string(),
			"vault".to_string(),
		),
		(
			"vault.security.banzaicloud.io/vault-skip-verify".to_string(),
			"true".to_string(),
		),
		(
			"vault.security.banzaicloud.io/vault-agent".to_string(),
			"false".to_string(),
		),
		(
			"vault.security.banzaicloud.io/vault-path".to_string(),
			"kubernetes".to_string(),
		),
	]
	.into_iter()
	.collect();

	Api::<Deployment>::namespaced(
		kube_client.clone(),
		&build_id.get_build_namespace(),
	)
	.create(
		&Default::default(),
		&Deployment {
			metadata: ObjectMeta {
				name: Some(service.name.to_string()),
				..Default::default()
			},
			spec: Some(DeploymentSpec {
				selector: LabelSelector {
					match_labels: Some(
						[("app".to_string(), service.name.to_string())].into(),
					),
					..Default::default()
				},
				template: PodTemplateSpec {
					metadata: Some(ObjectMeta {
						labels: Some(
							[("app".to_string(), service.name.to_string())]
								.into(),
						),
						annotations: Some(annotations),
						..Default::default()
					}),
					spec: Some(PodSpec {
						containers: vec![Container {
							name: service.name.to_string(),
							image: Some(service.image.clone()),
							image_pull_policy: Some("Always".to_string()),
							env,
							command: service.commands.as_ref().map(
								|commands| {
									vec![
										"sh".to_string(),
										"-ce".to_string(),
										commands.to_string(),
									]
								},
							),
							..Default::default()
						}],
						..Default::default()
					}),
				},
				..Default::default()
			}),
			..Default::default()
		},
	)
	.await?;

	Api::<Service>::namespaced(kube_client, &build_id.get_build_namespace())
		.create(
			&Default::default(),
			&Service {
				metadata: ObjectMeta {
					name: Some(service.name.to_string()),
					..Default::default()
				},
				spec: Some(ServiceSpec {
					selector: Some(
						[("app".to_string(), service.name.to_string())].into(),
					),
					ports: Some(vec![ServicePort {
						port: service.port,
						target_port: Some(IntOrString::Int(service.port)),
						..Default::default()
					}]),
					..Default::default()
				}),
				..Default::default()
			},
		)
		.await?;

	Ok(())
}

#[derive(Debug, PartialEq)]
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
