use std::{collections::BTreeMap, fmt::Display, time::Duration};

use api_models::{
	models::workspace::ci2::github::{BuildStatus, BuildStepStatus},
	utils::Uuid,
};
use chrono::Utc;
use eve_rs::AsError;
use k8s_openapi::{
	api::{
		batch::v1::{Job, JobSpec},
		core::v1::{
			Container,
			EnvVar,
			Namespace,
			PersistentVolumeClaimVolumeSource,
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
	api::{DeleteParams, ObjectMeta, PropagationPolicy},
	Api,
};
use serde::{Deserialize, Serialize};

use crate::{
	db,
	models::{
		ci::{self, file_format::EnvVarValue},
		rabbitmq::CIData,
	},
	service::{self, ext_traits::DeleteOpt},
	utils::{settings::Settings, Error},
	Database,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BuildId {
	pub repo_workspace_id: Uuid,
	pub repo_id: Uuid,
	pub build_num: i64,
}

impl BuildId {
	pub fn get_build_namespace(&self) -> String {
		format!("ci-{}-{}", self.repo_id, self.build_num)
	}

	pub fn get_pvc_name(&self) -> String {
		format!("pvc-{}-{}", self.repo_id, self.build_num)
	}
}

impl Display for BuildId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "ci-{}-{}", self.repo_id, self.build_num)
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BuildStepId {
	pub build_id: BuildId,
	pub step_id: i32,
}

impl BuildStepId {
	pub fn get_job_name(&self) -> String {
		format!(
			"ci-{}-{}-{}",
			self.build_id.repo_id, self.build_id.build_num, self.step_id
		)
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BuildStep {
	pub id: BuildStepId,
	pub image: String,
	pub env_vars: Vec<ci::file_format::EnvVar>,
	pub commands: Vec<String>,
}

async fn get_job_manifest(
	build_step: &BuildStep,
	config: &Settings,
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Job, Error> {
	let build_machine_type = db::get_build_machine_type_for_repo(
		&mut *connection,
		&build_step.id.build_id.repo_id,
	)
	.await?
	.status(500)?;

	let env = (!build_step.env_vars.is_empty()).then(|| {
		build_step
			.env_vars
			.iter()
			.map(|ci::file_format::EnvVar { name, value }| EnvVar {
				name: name.clone(),
				value: Some(match value {
					EnvVarValue::Value(value) => value.clone(),
					EnvVarValue::ValueFromSecret(from_secret) => format!(
						"vault:secret/data/{}/{}#data",
						build_step.id.build_id.repo_workspace_id, from_secret
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
		(
			"memory".to_string(),
			Quantity(format!("{:.1}G", (build_machine_type.ram as f64) / 4f64)),
		),
		(
			"cpu".to_string(),
			Quantity(format!("{:.1}", build_machine_type.cpu as f64)),
		),
	]
	.into_iter()
	.collect::<BTreeMap<_, _>>();

	let job = Job {
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
							mount_path: "/mnt/workdir".to_string(),
							name: "workdir".to_string(),
							..Default::default()
						}]),
						env,
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

	Ok(job)
}

pub async fn process_request(
	connection: &mut <Database as sqlx::Database>::Connection,
	request_data: CIData,
	config: &Settings,
) -> Result<(), Error> {
	let kube_client = service::get_kubernetes_config(config).await?;

	match request_data {
		CIData::BuildStep {
			build_step,
			request_id,
		} => {
			let build_step_job_name = build_step.id.get_job_name();
			let jobs_api = Api::<Job>::namespaced(
				kube_client,
				&build_step.id.build_id.get_build_namespace(),
			);
			let step_status = jobs_api
				.get_opt(&build_step_job_name)
				.await?
				.and_then(|job| job.status);

			enum Status {
				Errored,
				Completed,
				Running,
			}
			if let Some(status) = step_status {
				// update the stopped status for currently running steps,
				// dependent steps will get updated in their own messages

				// check whether the build has been stopped
				let build_status = db::get_build_status(
					&mut *connection,
					&build_step.id.build_id.repo_id,
					build_step.id.build_id.build_num,
				)
				.await?
				.unwrap_or(BuildStatus::Stopped); // TODO: handle none case

				if build_status == BuildStatus::Stopped {
					log::info!("request_id: {request_id} - Build step `{build_step_job_name}` stopped");
					jobs_api
						.delete_opt(
							&build_step_job_name,
							&DeleteParams {
								propagation_policy: Some(
									PropagationPolicy::Foreground,
								),
								..Default::default()
							},
						)
						.await?;
					db::update_build_step_status(
						connection,
						&build_step.id.build_id.repo_id,
						build_step.id.build_id.build_num,
						build_step.id.step_id,
						BuildStepStatus::Stopped,
					)
					.await?;
					db::update_build_step_finished_time(
						&mut *connection,
						&build_step.id.build_id.repo_id,
						build_step.id.build_id.build_num,
						build_step.id.step_id,
						&Utc::now(),
					)
					.await?;

					return Ok(());
				}

				let status = match (status.active.unwrap_or_default(), status.succeeded.unwrap_or_default(), status.failed.unwrap_or_default()) {
					(1, 0, 0) => Status::Running,
					(0, 1, 0) => Status::Completed,
					(0, 0, 1) => Status::Errored,
					(a, s, f) => unreachable!("expected one pod per job, invalid job status is obtained (active:{a}, succeeded:{s}, failed:{f})")
				};

				match status {
					Status::Errored => {
						log::info!("request_id: {request_id} - Build step `{build_step_job_name}` errored");
						jobs_api
							.delete_opt(
								&build_step_job_name,
								&DeleteParams {
									propagation_policy: Some(
										PropagationPolicy::Foreground,
									),
									..Default::default()
								},
							)
							.await?;
						db::update_build_step_status(
							connection,
							&build_step.id.build_id.repo_id,
							build_step.id.build_id.build_num,
							build_step.id.step_id,
							BuildStepStatus::Errored,
						)
						.await?;
						db::update_build_step_finished_time(
							&mut *connection,
							&build_step.id.build_id.repo_id,
							build_step.id.build_id.build_num,
							build_step.id.step_id,
							&Utc::now(),
						)
						.await?;
					}
					Status::Completed => {
						log::info!("request_id: {request_id} - Build step `{build_step_job_name}` succeeded");
						jobs_api
							.delete_opt(
								&build_step_job_name,
								&DeleteParams {
									propagation_policy: Some(
										PropagationPolicy::Foreground,
									),
									..Default::default()
								},
							)
							.await?;
						db::update_build_step_status(
							connection,
							&build_step.id.build_id.repo_id,
							build_step.id.build_id.build_num,
							build_step.id.step_id,
							BuildStepStatus::Succeeded,
						)
						.await?;
						db::update_build_step_finished_time(
							&mut *connection,
							&build_step.id.build_id.repo_id,
							build_step.id.build_id.build_num,
							build_step.id.step_id,
							&Utc::now(),
						)
						.await?;
					}
					Status::Running => {
						log::debug!("request_id: {request_id} - Waiting to update status of `{build_step_job_name}`");
						tokio::time::sleep(Duration::from_secs(2)).await;
						service::queue_create_ci_build_step(
							build_step,
							config,
							&request_id,
						)
						.await?;
					}
				}
			} else {
				let dependency_status = db::get_build_step_status(
					connection,
					&build_step.id.build_id.repo_id,
					build_step.id.build_id.build_num,
					// for now, only sequenctial build is supported,
					// so checking previous status is enough
					build_step.id.step_id - 1,
				)
				.await?;

				match dependency_status.unwrap_or(BuildStepStatus::Succeeded) {
					BuildStepStatus::Errored |
					BuildStepStatus::SkippedDepError => {
						log::info!("request_id: {request_id} - Build step `{build_step_job_name}` skipped as dependencies errored out");
						db::update_build_step_status(
							connection,
							&build_step.id.build_id.repo_id,
							build_step.id.build_id.build_num,
							build_step.id.step_id,
							BuildStepStatus::SkippedDepError,
						)
						.await?;
						db::update_build_step_started_time(
							&mut *connection,
							&build_step.id.build_id.repo_id,
							build_step.id.build_id.build_num,
							build_step.id.step_id,
							&Utc::now(),
						)
						.await?;
						db::update_build_step_finished_time(
							&mut *connection,
							&build_step.id.build_id.repo_id,
							build_step.id.build_id.build_num,
							build_step.id.step_id,
							&Utc::now(),
						)
						.await?;
					}
					BuildStepStatus::Stopped => {
						log::info!("request_id: {request_id} - Build step `{build_step_job_name}` stopped");
						db::update_build_step_status(
							connection,
							&build_step.id.build_id.repo_id,
							build_step.id.build_id.build_num,
							build_step.id.step_id,
							BuildStepStatus::Stopped,
						)
						.await?;
						db::update_build_step_started_time(
							&mut *connection,
							&build_step.id.build_id.repo_id,
							build_step.id.build_id.build_num,
							build_step.id.step_id,
							&Utc::now(),
						)
						.await?;
						db::update_build_step_finished_time(
							&mut *connection,
							&build_step.id.build_id.repo_id,
							build_step.id.build_id.build_num,
							build_step.id.step_id,
							&Utc::now(),
						)
						.await?;
					}
					BuildStepStatus::Succeeded => {
						log::info!("request_id: {request_id} - Starting build step `{build_step_job_name}`");
						jobs_api
							.create(
								&Default::default(),
								&get_job_manifest(
									&build_step,
									config,
									&mut *connection,
								)
								.await?,
							)
							.await?;
						db::update_build_step_status(
							connection,
							&build_step.id.build_id.repo_id,
							build_step.id.build_id.build_num,
							build_step.id.step_id,
							BuildStepStatus::Running,
						)
						.await?;
						db::update_build_step_started_time(
							&mut *connection,
							&build_step.id.build_id.repo_id,
							build_step.id.build_id.build_num,
							build_step.id.step_id,
							&Utc::now(),
						)
						.await?;
						service::queue_create_ci_build_step(
							build_step,
							config,
							&request_id,
						)
						.await?;
					}
					BuildStepStatus::Running |
					BuildStepStatus::WaitingToStart => {
						log::debug!("request_id: {request_id} - Waiting to create `{build_step_job_name}`");
						tokio::time::sleep(Duration::from_secs(2)).await;
						service::queue_create_ci_build_step(
							build_step,
							config,
							&request_id,
						)
						.await?;
					}
				}
			}
		}
		CIData::StopBuild {
			build_id,
			request_id,
		} => {
			log::info!("request_id: {request_id} - Build `{build_id}` stopped");
			db::update_build_status(
				&mut *connection,
				&build_id.repo_id,
				build_id.build_num,
				BuildStatus::Stopped,
			)
			.await?;
			db::update_build_finished_time(
				&mut *connection,
				&build_id.repo_id,
				build_id.build_num,
				&Utc::now(),
			)
			.await?;
		}
		CIData::CleanBuild {
			build_id,
			request_id,
		} => {
			let steps = db::list_build_steps_for_build(
				&mut *connection,
				&build_id.repo_id,
				build_id.build_num,
			)
			.await?;
			// for now sequential, so checking last status is enough
			let status = steps.last().map(|step| step.status.clone());

			match status.unwrap_or(BuildStepStatus::Succeeded) {
				BuildStepStatus::Errored | BuildStepStatus::SkippedDepError => {
					log::info!(
						"request_id: {request_id} - Build `{build_id}` errored"
					);
					Api::<Namespace>::all(kube_client)
						.delete_opt(
							&build_id.get_build_namespace(),
							&DeleteParams {
								propagation_policy: Some(
									PropagationPolicy::Foreground,
								),
								..Default::default()
							},
						)
						.await?;
					db::update_build_status(
						&mut *connection,
						&build_id.repo_id,
						build_id.build_num,
						BuildStatus::Errored,
					)
					.await?;
					db::update_build_finished_time(
						&mut *connection,
						&build_id.repo_id,
						build_id.build_num,
						&Utc::now(),
					)
					.await?;
				}
				BuildStepStatus::Succeeded => {
					log::info!(
						"request_id: {request_id} - Build `{build_id}` succeed"
					);
					Api::<Namespace>::all(kube_client)
						.delete_opt(
							&build_id.get_build_namespace(),
							&DeleteParams {
								propagation_policy: Some(
									PropagationPolicy::Foreground,
								),
								..Default::default()
							},
						)
						.await?;
					db::update_build_status(
						&mut *connection,
						&build_id.repo_id,
						build_id.build_num,
						BuildStatus::Succeeded,
					)
					.await?;
					db::update_build_finished_time(
						&mut *connection,
						&build_id.repo_id,
						build_id.build_num,
						&Utc::now(),
					)
					.await?;
				}
				BuildStepStatus::Running | BuildStepStatus::WaitingToStart => {
					log::debug!("request_id: {request_id} - Waiting to clean `{build_id}`");
					tokio::time::sleep(Duration::from_secs(5)).await;
					service::queue_clean_ci_build_pipeline(
						build_id,
						config,
						&request_id,
					)
					.await?;
				}
				BuildStepStatus::Stopped => {
					log::debug!("request_id: {request_id} - Cleaning stopped build `{build_id}`");
					Api::<Namespace>::all(kube_client)
						.delete_opt(
							&build_id.get_build_namespace(),
							&DeleteParams {
								propagation_policy: Some(
									PropagationPolicy::Foreground,
								),
								..Default::default()
							},
						)
						.await?;
				}
			}
		}
	}
	Ok(())
}
