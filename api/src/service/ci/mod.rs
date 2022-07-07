use std::fmt::Display;

use api_models::utils::Uuid;
use k8s_openapi::{
	api::{
		batch::v1::{Job, JobSpec},
		core::v1::{
			Container,
			EnvVar,
			PersistentVolumeClaim,
			PersistentVolumeClaimSpec,
			PersistentVolumeClaimVolumeSource,
			Pod,
			PodSpec,
			PodTemplateSpec,
			ResourceRequirements,
			Volume,
			VolumeMount,
		},
	},
	apimachinery::pkg::api::resource::Quantity,
};
use kube::{
	api::{ObjectMeta, PostParams},
	Api,
};
use serde_json::json;

use crate::{
	models::{CiFlow, Kind, Step},
	rabbitmq::{self, BuildId, BuildStepId},
	service::{self, queue_clone_ci_repo},
	utils::{settings::Settings, Error},
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
	branch_name: &str,
	netrc: Option<Netrc>,
	build_id: BuildId,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::debug!(
		"request_id: {request_id} - Create a pod to run custom ci commands"
	);
	let kube_client = service::get_kubernetes_config(config).await?;

	let BuildId {
		workspace_id,
		repo_id,
		build_num,
	} = build_id;

	log::debug!("request_id: {request_id} - Creating pvc storage");
	// create a pvc for storage
	let pvc_name = format!("ci-{repo_id}-{build_num}");
	Api::<PersistentVolumeClaim>::namespaced(kube_client, "patrci")
		.create(
			&Default::default(),
			&PersistentVolumeClaim {
				metadata: ObjectMeta {
					name: Some(pvc_name.clone()),
					..Default::default()
				},
				spec: Some(PersistentVolumeClaimSpec {
					access_modes: Some(vec!["ReadWriteOnce".to_string()]),
					storage_class_name: Some("csi-s3".to_string()),
					resources: Some(ResourceRequirements {
						requests: Some(
							[(
								"storage".to_string(),
								Quantity("1G".to_string()), // TODO
							)]
							.into(),
						),
						..Default::default()
					}),
					..Default::default()
				}),
				..Default::default()
			},
		)
		.await?;

	// first clone the repo
	let git_clone_job = Job {
		metadata: ObjectMeta {
			name: Some(format!("ci-{repo_id}-{build_num}-0")),
			..Default::default()
		},
		spec: Some(JobSpec {
			backoff_limit: Some(1), // TODO
			template: PodTemplateSpec {
				spec: Some(PodSpec {
					containers: vec![Container {
						image: Some("alpine/git".to_string()),
						image_pull_policy: Some("Always".to_string()),
						name: "git-clone".to_string(),
						volume_mounts: Some(vec![VolumeMount {
							mount_path: "/mnt/workdir".to_string(),
							name: "workdir".to_string(),
							..Default::default()
						}]),
						command: Some(vec![
							"sh".to_string(),
							"-ce".to_string(),
							[
								&format!(r#"echo "{}" > ~/.netrc"#, netrc.map_or("".to_string(), |netrc| netrc.to_string())),
								r#"cd "/mnt/workdir/""#,
								"set -x",
								&format!(
									r#"git clone --filter=tree:0 --single-branch --branch="{branch_name}" "{repo_clone_url}""#
								),
							]
							.join("\n"),
						]),
						..Default::default()
					}],
					volumes: Some(vec![Volume {
						name: "workdir".to_string(),
						persistent_volume_claim: Some(
							PersistentVolumeClaimVolumeSource {
								claim_name: pvc_name.clone(),
								..Default::default()
							},
						),
						..Default::default()
					}]),
					restart_policy: Some("Never".to_string()),
					..Default::default()
				}),
				..Default::default()
			},
			// ttl seconds buffer time should accomodate mq check time
			ttl_seconds_after_finished: Some(120), // TODO
			..Default::default()
		}),
		..Default::default()
	};
	log::debug!("request_id: {request_id} - Creating git clone job");
	service::queue_clone_ci_repo(
		BuildStepId {
			step_id: 0,
			workspace_id: workspace_id.clone(),
			repo_id: repo_id.clone(),
			build_num,
		},
		git_clone_job,
		config,
		request_id,
	)
	.await?;

	log::debug!("request_id: {request_id} - Creating jobs for each step");
	let Kind::Pipeline(pipeline) = ci_flow.kind;
	for (
		step_id, // TODO
		Step {
			name: _name,
			image,
			commands,
			env,
		},
	) in pipeline.steps.into_iter().enumerate()
	{
		let step_id = 1 + step_id as i32;
		let job = Job {
			metadata: ObjectMeta {
				name: Some(format!("ci-{repo_id}-{build_num}-{step_id}")),
				..Default::default()
			},
			spec: Some(JobSpec {
				backoff_limit: Some(1), // TODO
				template: PodTemplateSpec {
					spec: Some(PodSpec {
						containers: vec![Container {
							image: Some(image),
							image_pull_policy: Some("Always".to_string()),
							name: step_id.to_string(),
							volume_mounts: Some(vec![VolumeMount {
								mount_path: "/mnt/workdir".to_string(),
								name: "workdir".to_string(),
								..Default::default()
							}]),
							env: Some(
								env.into_iter()
									.map(|env| EnvVar {
										name: env.name,
										value: Some(env.value),
										..Default::default()
									})
									.collect(),
							),
							command: Some(vec![
								"sh".to_string(),
								"-ce".to_string(),
								[
									format!(r#"cd "/mnt/workdir/{repo_name}""#),
									"set -x".to_owned(),
								]
								.into_iter()
								.chain(commands.into_iter())
								.collect::<Vec<_>>()
								.join("\n"),
							]),
							..Default::default()
						}],
						volumes: Some(vec![Volume {
							name: "workdir".to_string(),
							persistent_volume_claim: Some(
								PersistentVolumeClaimVolumeSource {
									claim_name: pvc_name.clone(),
									..Default::default()
								},
							),
							..Default::default()
						}]),
						restart_policy: Some("Never".to_string()),
						..Default::default()
					}),
					..Default::default()
				},
				// ttl seconds buffer time should accomodate mq check time
				ttl_seconds_after_finished: Some(120), // TODO
				..Default::default()
			}),
			..Default::default()
		};
		service::queue_create_build_step(
			BuildStepId {
				workspace_id: workspace_id.clone(),
				repo_id: repo_id.clone(),
				build_num,
				step_id,
			},
			job,
			config,
			request_id,
		)
		.await?;
	}

	log::debug!("request_id: {request_id} - Creating clean build job");
	service::queue_clean_build_pipeline(
		BuildId {
			workspace_id,
			repo_id,
			build_num,
		},
		config,
		request_id,
	)
	.await?;

	log::debug!("successfully created a ci pipeline in k8s");

	Ok(())
}
