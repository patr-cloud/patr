use std::time::Duration;

use api_models::{models::workspace::ci2::github::EnvVariable, utils::Uuid};
use k8s_openapi::api::{
	batch::v1::{Job, JobSpec},
	core::v1::{
		Container,
		EnvVar,
		PersistentVolumeClaim,
		PersistentVolumeClaimVolumeSource,
		PodSpec,
		PodTemplateSpec,
		Volume,
		VolumeMount,
	},
};
use kube::{
	api::{DeleteParams, ObjectMeta, PropagationPolicy},
	Api,
};
use serde::{Deserialize, Serialize};

use crate::{
	db,
	models::rabbitmq::CIData,
	service,
	utils::{settings::Settings, Error},
	Database,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BuildId {
	pub workspace_id: Uuid,
	pub repo_id: Uuid,
	pub build_num: i64,
}

impl BuildId {
	pub fn get_pvc_name(&self) -> String {
		format!("ci-{}-{}", self.repo_id, self.build_num)
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
	pub env_vars: Vec<EnvVariable>,
	pub commands: Vec<String>,
}

impl BuildStep {
	pub fn get_job_manifest(&self) -> Job {
		Job {
			metadata: ObjectMeta {
				name: Some(self.id.get_job_name()),
				..Default::default()
			},
			spec: Some(JobSpec {
				backoff_limit: Some(0),
				template: PodTemplateSpec {
					spec: Some(PodSpec {
						containers: vec![Container {
							image: Some(self.image.clone()),
							image_pull_policy: Some("Always".to_string()),
							name: "build-step".to_string(),
							volume_mounts: Some(vec![VolumeMount {
								mount_path: "/mnt/workdir".to_string(),
								name: "workdir".to_string(),
								..Default::default()
							}]),
							env: Some(
								self.env_vars
									.iter()
									.map(|env| EnvVar {
										name: env.name.clone(),
										value: Some(env.value.clone()),
										..Default::default()
									})
									.collect(),
							),
							command: Some(vec![
								"sh".to_string(),
								"-ce".to_string(),
								self.commands.join("\n"),
							]),
							..Default::default()
						}],
						volumes: Some(vec![Volume {
							name: "workdir".to_string(),
							persistent_volume_claim: Some(
								PersistentVolumeClaimVolumeSource {
									claim_name: self.id.build_id.get_pvc_name(),
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
				..Default::default()
			}),
			..Default::default()
		}
	}
}

enum Status {
	Errored,
	Completed,
	Running,
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
				build_step.id.build_id.workspace_id.as_str(),
			);
			let step_status = jobs_api
				.get_opt(&build_step_job_name)
				.await?
				.and_then(|job| job.status);

			if let Some(status) = step_status {
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
							.delete(
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
							"errored",
						)
						.await?
					}
					Status::Completed => {
						log::info!("request_id: {request_id} - Build step `{build_step_job_name}` succeeded");
						jobs_api
							.delete(
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
							"success",
						)
						.await?
					}
					Status::Running => {
						log::debug!("request_id: {request_id} - Waiting to update status of `{build_step_job_name}`");
						tokio::time::sleep(Duration::from_secs(5)).await;
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
					build_step.id.step_id - 1, /* since sequential, checking
					                            * previous status is okay
					                            * for now */
				)
				.await?;

				let dependency_status = match dependency_status
					.unwrap_or_else(|| "success".to_string()) // for clone command it will be None
					.as_str()
				{
					"running" | "waiting_to_start" => Status::Running,
					"errored" | "skipped-parent_error" => Status::Errored,
					"success" => Status::Completed,
					unknown => {
						unreachable!("invalid dependency status `{unknown}`")
					}
				};

				match dependency_status {
					Status::Errored => {
						log::info!("request_id: {request_id} - Build step `{build_step_job_name}` skipped as dependencies errored out");
						db::update_build_step_status(
							connection,
							&build_step.id.build_id.repo_id,
							build_step.id.build_id.build_num,
							build_step.id.step_id,
							"skipped-parent_error",
						)
						.await?
					}
					Status::Completed => {
						log::info!("request_id: {request_id} - Starting build step `{build_step_job_name}`");
						jobs_api
							.create(
								&Default::default(),
								&build_step.get_job_manifest(),
							)
							.await?;
						db::update_build_step_status(
							connection,
							&build_step.id.build_id.repo_id,
							build_step.id.build_id.build_num,
							build_step.id.step_id,
							"running",
						)
						.await?;
						service::queue_create_ci_build_step(
							build_step,
							config,
							&request_id,
						)
						.await?;
					}
					Status::Running => {
						log::debug!("request_id: {request_id} - Waiting to create `{build_step_job_name}`");
						tokio::time::sleep(Duration::from_secs(5)).await;
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
		CIData::CleanBuild {
			build_id,
			request_id,
		} => {
			let steps = db::get_build_steps_for_build(
				&mut *connection,
				&build_id.repo_id,
				build_id.build_num,
			)
			.await?;
			// for now sequential, so checking last status is enough
			let status = steps.last().map(|step| step.step_status.clone());

			let status = match status
				.unwrap_or_else(|| "unknown".to_string())
				.as_str()
			{
				"running" | "waiting_to_start" => Status::Running,
				"errored" | "skipped-parent_error" => Status::Errored,
				"success" => Status::Completed,
				unknown => {
					unreachable!("invalid dependency status `{unknown}`")
				}
			};

			let pvc_name = build_id.get_pvc_name();
			match status {
				Status::Errored => {
					log::info!(
						"request_id: {request_id} - Build `{pvc_name}` errored"
					);
					Api::<PersistentVolumeClaim>::namespaced(
						kube_client,
						build_id.workspace_id.as_str(),
					)
					.delete(&pvc_name, &Default::default())
					.await?;
					db::update_build_status(
						&mut *connection,
						&build_id.repo_id,
						build_id.build_num,
						"errored",
					)
					.await?;
				}
				Status::Completed => {
					log::info!(
						"request_id: {request_id} - Build `{pvc_name}` succeed"
					);
					Api::<PersistentVolumeClaim>::namespaced(
						kube_client,
						build_id.workspace_id.as_str(),
					)
					.delete(&pvc_name, &Default::default())
					.await?;
					db::update_build_status(
						&mut *connection,
						&build_id.repo_id,
						build_id.build_num,
						"success",
					)
					.await?;
				}
				Status::Running => {
					log::debug!("request_id: {request_id} - Waiting to clean `{pvc_name}`");
					tokio::time::sleep(Duration::from_secs(10)).await;
					service::queue_clean_ci_build_pipeline(
						build_id,
						config,
						&request_id,
					)
					.await?;
				}
			}
		}
	}
	Ok(())
}
