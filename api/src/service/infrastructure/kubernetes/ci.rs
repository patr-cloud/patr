use std::collections::BTreeMap;

use api_models::{models::ci::file_format::EnvVarValue, utils::Uuid};
use k8s_openapi::{
	api::{
		apps::v1::{Deployment, DeploymentSpec},
		batch::v1::{Job, JobSpec},
		core::v1::{
			Container,
			EnvVar,
			PersistentVolumeClaim,
			PersistentVolumeClaimSpec,
			PersistentVolumeClaimVolumeSource,
			PodSpec,
			PodTemplateSpec,
			ResourceRequirements,
			Service,
			ServicePort,
			ServiceSpec,
			Volume,
			VolumeMount,
		},
	},
	apimachinery::pkg::{
		api::resource::Quantity,
		apis::meta::v1::LabelSelector,
		util::intstr::IntOrString,
	},
};
use kube::{
	api::{DeleteParams, PropagationPolicy},
	config::Kubeconfig,
	core::ObjectMeta,
	Api,
};

use crate::{
	models::ci::{Commit, EventType, PullRequest, Tag},
	rabbitmq::BuildStep,
	service::ext_traits::DeleteOpt,
	utils::{settings::Settings, Error},
};

pub async fn create_ci_job_in_kubernetes(
	namespace_name: &str,
	build_step: &BuildStep,
	ram_in_mb: u32,
	cpu_in_milli: u32,
	event_type: &EventType,
	kubeconfig: Kubeconfig,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let client = super::get_kubernetes_client(kubeconfig).await?;
	log::trace!(
		"request_id: {} - creating ci job {} in namespace {}",
		request_id,
		build_step.id.get_job_name(),
		namespace_name,
	);

	let annotations = [
		(
			"vault.security.banzaicloud.io/vault-addr".to_string(),
			config.vault.upstream_host.clone(),
		),
		(
			"vault.security.banzaicloud.io/vault-role".to_string(),
			"vault".to_string(),
		),
		(
			"vault.security.banzaicloud.io/vault-skip-verify".to_string(),
			"false".to_string(),
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

	let build_machine_type = [
		("memory".to_string(), Quantity(format!("{}M", ram_in_mb))),
		("cpu".to_string(), Quantity(format!("{}m", cpu_in_milli))),
	]
	.into_iter()
	.collect::<BTreeMap<_, _>>();

	let job_manifest = Job {
		metadata: ObjectMeta {
			name: Some(build_step.id.get_job_name()),
			..Default::default()
		},
		spec: Some(JobSpec {
			backoff_limit: Some(0),
			template: PodTemplateSpec {
				metadata: Some(ObjectMeta {
					annotations: Some(annotations),
					..Default::default()
				}),
				spec: Some(PodSpec {
					containers: vec![Container {
						image: Some(build_step.image.clone()),
						image_pull_policy: Some("Always".to_string()),
						name: "build-step".to_string(),
						volume_mounts: Some(vec![VolumeMount {
							mount_path: "/workdir".to_string(),
							name: "workdir".to_string(),
							..Default::default()
						}]),
						env: Some(get_env_variables_for_build(
							build_step, event_type,
						)),
						command: Some(vec![
							"sh".to_string(),
							"-ce".to_string(),
							build_step.commands.join("\n"),
						]),
						resources: Some(ResourceRequirements {
							limits: Some(build_machine_type),
							..Default::default()
						}),
						..Default::default()
					}],
					volumes: Some(vec![Volume {
						name: "workdir".to_string(),
						persistent_volume_claim: Some(
							PersistentVolumeClaimVolumeSource {
								claim_name: build_step
									.id
									.build_id
									.get_pvc_name(),
								..Default::default()
							},
						),
						..Default::default()
					}]),
					restart_policy: Some("Never".to_string()),
					..Default::default()
				}),
			},
			..Default::default()
		}),
		..Default::default()
	};

	Api::<Job>::namespaced(client, namespace_name)
		.create(&Default::default(), &job_manifest)
		.await?;

	log::trace!("request_id: {} - created job", request_id);
	Ok(())
}

fn get_env_variables_for_build(
	build_step: &BuildStep,
	event_type: &EventType,
) -> Vec<EnvVar> {
	let patr_ci_default_envs = [
		("CI", "true"),
		("PATR", "true"),
		("PATR_CI", "true"),
		("PATR_CI_WORKDIR", "/workdir"),
	]
	.into_iter()
	.map(|(name, value)| (name.to_string(), value.to_string()));

	let build_number_env = [(
		"PATR_CI_BUILD_NUMBER".to_string(),
		build_step.id.build_id.build_num.to_string(),
	)];

	let event_type_envs = match event_type {
		EventType::Commit(Commit {
			commit_sha,
			commit_message,
			committed_branch_name,
			..
		}) => {
			let mut envs = vec![
				("PATR_CI_EVENT_TYPE".to_string(), "commit".to_string()),
				("PATR_CI_COMMIT_SHA".to_string(), commit_sha.to_string()),
				(
					"PATR_CI_BRANCH".to_string(),
					committed_branch_name.to_string(),
				),
			];

			if let Some(commit_message) = commit_message {
				envs.push((
					"PATR_CI_COMMIT_MESSAGE".to_string(),
					commit_message.to_string(),
				))
			}

			envs
		}
		EventType::Tag(Tag {
			commit_sha,
			tag_name,
			commit_message,
			..
		}) => {
			let mut envs = vec![
				("PATR_CI_EVENT_TYPE".to_string(), "tag".to_string()),
				("PATR_CI_COMMIT_SHA".to_string(), commit_sha.to_string()),
				("PATR_CI_TAG".to_string(), tag_name.to_string()),
			];

			if let Some(commit_message) = commit_message {
				envs.push((
					"PATR_CI_COMMIT_MESSAGE".to_string(),
					commit_message.to_string(),
				))
			}

			envs
		}
		EventType::PullRequest(PullRequest {
			commit_sha,
			pr_number,
			pr_title,
			to_be_committed_branch_name,
			..
		}) => vec![
			("PATR_CI_EVENT_TYPE".to_string(), "pull_request".to_string()),
			("PATR_CI_COMMIT_SHA".to_string(), commit_sha.to_string()),
			(
				"PATR_CI_BRANCH".to_string(),
				to_be_committed_branch_name.to_string(),
			),
			(
				"PATR_CI_PULL_REQUEST_TITLE".to_string(),
				pr_title.to_string(),
			),
			(
				"PATR_CI_PULL_REQUEST_NUMBER".to_string(),
				pr_number.to_string(),
			),
		],
	};

	let user_envs = build_step.env_vars.iter().map(|(name, value)| {
		(
			name.clone(),
			match value {
				EnvVarValue::Value(value) => value.clone(),
				EnvVarValue::ValueFromSecret { from_secret } => format!(
					"vault:secret/data/{}/{}#data",
					build_step.id.build_id.repo_workspace_id, from_secret
				),
			},
		)
	});

	patr_ci_default_envs
		.chain(build_number_env)
		.chain(event_type_envs)
		.chain(user_envs)
		.map(|(name, value)| EnvVar {
			name,
			value: Some(value),
			..Default::default()
		})
		.collect()
}

pub enum JobStatus {
	Errored,
	Completed,
	Running,
}

pub async fn get_ci_job_status_in_kubernetes(
	namespace_name: &str,
	job_name: &str,
	kubeconfig: Kubeconfig,
	request_id: &Uuid,
) -> Result<Option<JobStatus>, Error> {
	let client = super::get_kubernetes_client(kubeconfig).await?;

	let status = Api::<Job>::namespaced(client, namespace_name)
		.get_opt(job_name)
		.await?
		.and_then(|job| job.status)
		.map(|status| {
			(
				status.active.unwrap_or_default(),
				status.succeeded.unwrap_or_default(),
				status.failed.unwrap_or_default(),
			)
		})
		.map(|status| match status {
			(1, 0, 0) => JobStatus::Running,
			(0, 1, 0) => JobStatus::Completed,
			(0, 0, 1) => JobStatus::Errored,
			(a, s, f) => {
				log::info!(
					"request_id: {} - Unexpected job status (active:{}, succeeded:{}, failed:{})",
					request_id,
					a,
					s,
					f
				);
				JobStatus::Errored
			}
		});

	Ok(status)
}

pub async fn create_pvc_for_workspace(
	namespace_name: &str,
	pvc_name: &str,
	volume_in_mb: u32,
	kubeconfig: Kubeconfig,
	request_id: &Uuid,
) -> Result<(), Error> {
	let client = super::get_kubernetes_client(kubeconfig).await?;

	log::trace!(
		"request_id: {} - creating pvc {} of size {} in namespace {}",
		request_id,
		pvc_name,
		volume_in_mb,
		namespace_name,
	);

	let pvc_spec = PersistentVolumeClaimSpec {
		access_modes: Some(vec!["ReadWriteOnce".to_string()]),
		resources: Some(ResourceRequirements {
			requests: Some(
				[(
					"storage".to_string(),
					Quantity(format!("{}M", volume_in_mb)),
				)]
				.into(),
			),
			..Default::default()
		}),
		volume_mode: Some("Filesystem".to_string()),
		..Default::default()
	};

	Api::<PersistentVolumeClaim>::namespaced(client, namespace_name)
		.create(
			&Default::default(),
			&PersistentVolumeClaim {
				metadata: ObjectMeta {
					name: Some(pvc_name.to_string()),
					..Default::default()
				},
				spec: Some(pvc_spec),
				..Default::default()
			},
		)
		.await?;

	log::trace!("request_id: {} - pvc created", request_id);

	Ok(())
}

pub async fn delete_kubernetes_job(
	namespace_name: &str,
	job_name: &str,
	kubeconfig: Kubeconfig,
	request_id: &Uuid,
) -> Result<(), Error> {
	let client = super::get_kubernetes_client(kubeconfig).await?;
	log::trace!(
		"request_id: {} - deleting job {} in namespace {}",
		request_id,
		job_name,
		namespace_name
	);

	let jobs_api = Api::<Job>::namespaced(client, namespace_name);
	jobs_api
		.delete_opt(
			job_name,
			&DeleteParams {
				propagation_policy: Some(PropagationPolicy::Foreground),
				..Default::default()
			},
		)
		.await?;

	log::trace!("request_id: {} - deleted job", request_id);
	Ok(())
}

pub async fn create_background_service_for_ci_in_kubernetes(
	namespace_name: &str,
	repo_workspace_name: &str,
	service: api_models::models::ci::file_format::Service,
	kubeconfig: Kubeconfig,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let client = super::get_kubernetes_client(kubeconfig).await?;
	log::trace!(
		"request_id: {} - creating background ci service {} in namespace {}",
		request_id,
		service.name,
		namespace_name
	);

	let env = (!service.environment.is_empty()).then(|| {
		service
			.environment
			.iter()
			.map(|(name, value)| EnvVar {
				name: name.clone(),
				value: Some(match value {
					EnvVarValue::Value(value) => value.clone(),
					EnvVarValue::ValueFromSecret { from_secret } => format!(
						"vault:secret/data/{}/{}#data",
						repo_workspace_name, from_secret
					),
				}),
				..Default::default()
			})
			.collect()
	});

	let annotations = [
		(
			"vault.security.banzaicloud.io/vault-addr".to_string(),
			config.vault.host.clone(),
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

	Api::<Deployment>::namespaced(client.clone(), namespace_name)
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
							[("app".to_string(), service.name.to_string())]
								.into(),
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
								command: service.commands.map(|command| {
									vec![
										"sh".to_string(),
										"-ce".to_string(),
										Vec::from(command).join("\n"),
									]
								}),
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

	Api::<Service>::namespaced(client, namespace_name)
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
						port: service.port as i32,
						target_port: Some(IntOrString::Int(
							service.port as i32,
						)),
						..Default::default()
					}]),
					..Default::default()
				}),
				..Default::default()
			},
		)
		.await?;

	log::trace!("request_id: {} - created background service", request_id);
	Ok(())
}
