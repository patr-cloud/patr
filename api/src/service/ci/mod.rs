use std::fmt::Display;

use api_models::utils::Uuid;
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
	db,
	models::ci::{
		self,
		file_format::{CiFlow, EnvVarValue, Step},
	},
	rabbitmq::{BuildId, BuildStep, BuildStepId},
	service,
	utils::{settings::Settings, Error},
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

pub async fn create_ci_pipeline(
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
