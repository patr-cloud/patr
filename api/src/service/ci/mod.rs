use std::{
	collections::{HashMap, HashSet},
	fmt::Display,
};

use api_models::{
	models::workspace::ci2::github::{BuildStatus, BuildStepStatus},
	utils::Uuid,
};
use chrono::Utc;
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
	db::{self, Repository},
	models::ci::{
		self,
		file_format::{CiFlow, EnvVarValue, Step},
	},
	rabbitmq::{BuildId, BuildStep, BuildStepId},
	service,
	utils::{settings::Settings, Error, EveContext},
	Database,
};

pub mod github;

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

async fn create_ci_pipeline_in_k8s(
	ci_flow: CiFlow,
	repo_clone_url: &str,
	repo_name: &str,
	commit_sha: &str,
	netrc: Option<Netrc>,
	build_id: BuildId,
	config: &Settings,
	connection: &mut <Database as sqlx::Database>::Connection,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::debug!("request_id: {request_id} - Creating a ci pipeline for build `{build_id}`");

	let build_machine_type = db::get_build_machine_type_for_repo(
		&mut *connection,
		&build_id.repo_id,
	)
	.await?
	.status(500)?;

	let kube_client = service::get_kubernetes_config(config).await?;

	// create a namespace for each build
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

	// create pvc for storage
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
			&build_id,
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
				format!(r#"git checkout "{commit_sha}""#),
			],
		},
		config,
		request_id,
	)
	.await?;

	// queue build steps
	for (
		step_id, // TODO
		Step {
			name: _,
			image,
			commands,
			env,
		},
	) in pipeline.steps.into_iter().enumerate()
	{
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
						build_id.workspace_id, from_secret
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

pub async fn create_ci_build(
	context: &mut EveContext,
	repo: &Repository,
	git_ref: &str,
	git_commit: &str,
	ci_file: bytes::Bytes,
	access_token: &str,
	repo_clone_url: &str,
	repo_name: &str,
	request_id: Uuid,
) -> Result<i64, eve_rs::Error<()>> {
	let build_num = db::generate_new_build_for_repo(
		context.get_database_connection(),
		&repo.id,
		git_ref,
		git_commit,
		BuildStatus::Running,
		&Utc::now(),
	)
	.await?;

	let mut ci_flow: CiFlow = serde_yaml::from_slice(ci_file.as_ref())?;
	let CiFlow::Pipeline(pipeline) = ci_flow.clone();

	// validate the ci file
	if !is_names_unique(&ci_flow) {
		log::info!(
			"request_id: {request_id} - Invalid ci config file, marking build as errored"
		);
		db::update_build_status(
			context.get_database_connection(),
			&repo.id,
			build_num,
			BuildStatus::Errored,
		)
		.await?;

		return Ok(build_num);
	}

	if !find_and_replace_secret_names(
		context.get_database_connection(),
		&mut ci_flow,
		&repo.workspace_id,
	)
	.await?
	{
		log::info!(
			"request_id: {request_id} - Invalid secret name given, marking build as errored"
		);
		db::update_build_status(
			context.get_database_connection(),
			&repo.id,
			build_num,
			BuildStatus::Errored,
		)
		.await?;

		return Ok(build_num);
	};

	// add cloning as a step
	db::add_ci_steps_for_build(
		context.get_database_connection(),
		&repo.id,
		build_num,
		0,
		"git-clone",
		"",
		vec![],
		BuildStepStatus::WaitingToStart,
	)
	.await?;

	for (
		step_count,
		Step {
			name,
			image,
			commands,
			env: _,
		},
	) in pipeline.steps.into_iter().enumerate()
	{
		db::add_ci_steps_for_build(
			context.get_database_connection(),
			&repo.id,
			build_num,
			step_count as i32 + 1,
			&name,
			&image,
			vec![commands.to_string()],
			BuildStepStatus::WaitingToStart,
		)
		.await?;
	}

	context.commit_database_transaction().await?;

	// TODO: make more generic
	let netrc = Netrc {
		machine: "github.com".to_string(),
		login: "oauth".to_string(),
		password: access_token.to_string(),
	};

	create_ci_pipeline_in_k8s(
		ci_flow,
		repo_clone_url,
		repo_name,
		git_commit,
		Some(netrc),
		BuildId {
			workspace_id: repo.workspace_id.clone(),
			repo_id: repo.id.clone(),
			build_num,
		},
		&context.get_state().config.clone(),
		context.get_database_connection(),
		&request_id,
	)
	.await?;

	Ok(build_num)
}

fn is_names_unique(ci_flow: &CiFlow) -> bool {
	let CiFlow::Pipeline(pipeline) = ci_flow;

	let mut step_names = HashSet::new();
	for step in &pipeline.steps {
		if !step_names.insert(step.name.as_str()) {
			return false;
		}
	}

	let mut service_names = HashSet::new();
	for service in &pipeline.services {
		if !service_names.insert(service.name.as_str()) {
			return false;
		}
	}

	true
}

async fn find_and_replace_secret_names(
	connection: &mut <Database as sqlx::Database>::Connection,
	ci_flow: &mut CiFlow,
	workspace_id: &Uuid,
) -> Result<bool, Error> {
	let workspace_secrets =
		db::get_all_secrets_in_workspace(connection, workspace_id)
			.await?
			.into_iter()
			.map(|secret| (secret.name, secret.id))
			.collect::<HashMap<_, _>>();

	let CiFlow::Pipeline(pipeline) = ci_flow;

	for service in &mut pipeline.services {
		for env in &mut service.env {
			if let EnvVarValue::ValueFromSecret(secret_name) = &mut env.value {
				if let Some(secret_id) = workspace_secrets.get(&*secret_name) {
					*secret_name = secret_id.to_string();
				} else {
					return Ok(false);
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
					return Ok(false);
				}
			}
		}
	}

	Ok(true)
}
