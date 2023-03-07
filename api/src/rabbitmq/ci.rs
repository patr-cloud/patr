use std::{collections::BTreeMap, fmt::Display, time::Duration};

use api_models::{
	models::{
		ci::file_format::EnvVarValue,
		workspace::{
			ci::git_provider::{BuildStatus, BuildStepStatus},
			region::RegionStatus,
		},
	},
	utils::Uuid,
};
use chrono::Utc;
use eve_rs::AsError;
use serde::{Deserialize, Serialize};
use sqlx::{types::Json, Acquire};

use crate::{
	db,
	models::{ci::github::CommitStatus, rabbitmq::CIData},
	service::{self, JobStatus},
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
	pub env_vars: BTreeMap<String, EnvVarValue>,
	pub commands: Vec<String>,
}

pub async fn process_request(
	connection: &mut <Database as sqlx::Database>::Connection,
	request_data: CIData,
	config: &Settings,
) -> Result<(), Error> {
	match request_data {
		CIData::CheckAndStartBuild {
			build_id,
			services,
			work_steps,
			event_type,
			request_id,
		} => {
			let mut connection = connection.begin().await?;

			let repo_id = build_id.repo_id.clone();
			let build_num = build_id.build_num;

			// get the build status with lock, so that it won't be updated in
			// routes until this rabbitmq msg is processed.
			let build_status = db::get_build_status_for_update(
				&mut connection,
				&repo_id,
				build_num,
			)
			.await?
			.status(500)?;

			if build_status == BuildStatus::Cancelled {
				// build has been cancelled,
				// so update the build steps and
				// then discard this msg
				log::debug!("request_id: {request_id} - Updating cancelled build steps `{build_id}`");
				for step_id in 0..=work_steps.len() {
					db::update_build_step_status(
						&mut connection,
						&repo_id,
						build_num,
						step_id as i32,
						BuildStepStatus::Cancelled,
					)
					.await?;
					db::update_build_step_finished_time(
						&mut connection,
						&repo_id,
						build_num,
						step_id as i32,
						&Utc::now(),
					)
					.await?;
				}
			} else {
				let runner_available = db::is_runner_available_to_start_build(
					&mut connection,
					&repo_id,
					build_num,
				)
				.await?;

				if runner_available {
					// runner is available spawn the build step
					log::debug!("request_id: {request_id} - Runner is available to start build `{build_id}`");
					service::add_build_steps_in_k8s(
						&mut connection,
						config,
						&build_id,
						services,
						work_steps,
						event_type,
						&request_id,
					)
					.await?;
				} else {
					log::debug!("request_id: {request_id} - Waiting for runner to start build `{build_id}`");
					tokio::time::sleep(Duration::from_secs(1)).await;
					service::queue_check_and_start_ci_build(
						build_id,
						services,
						work_steps,
						event_type,
						config,
						&request_id,
					)
					.await?;
				}
			}

			connection.commit().await?;
		}
		CIData::BuildStep {
			build_step,
			request_id,
		} => {
			let build_namespace = build_step.id.build_id.get_build_namespace();
			let build_step_job_name = build_step.id.get_job_name();

			let kubeconfig = service::get_kubeconfig_for_ci_build(
				connection,
				&build_step.id.build_id,
			)
			.await?;

			let step_status = service::get_ci_job_status_in_kubernetes(
				&build_namespace,
				&build_step_job_name,
				kubeconfig.clone(),
				&request_id,
			)
			.await?;

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
				.unwrap_or(BuildStatus::Errored);

				if build_status == BuildStatus::Cancelled {
					log::info!("request_id: {request_id} - Build step `{build_step_job_name}` stopped");
					service::delete_kubernetes_job(
						&build_namespace,
						&build_step_job_name,
						kubeconfig.clone(),
						&request_id,
					)
					.await?;
					db::update_build_step_status(
						connection,
						&build_step.id.build_id.repo_id,
						build_step.id.build_id.build_num,
						build_step.id.step_id,
						BuildStepStatus::Cancelled,
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

				match status {
					JobStatus::Errored => {
						log::info!("request_id: {request_id} - Build step `{build_step_job_name}` errored");
						service::delete_kubernetes_job(
							&build_namespace,
							&build_step_job_name,
							kubeconfig,
							&request_id,
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
					JobStatus::Completed => {
						log::info!("request_id: {request_id} - Build step `{build_step_job_name}` succeeded");
						service::delete_kubernetes_job(
							&build_namespace,
							&build_step_job_name,
							kubeconfig,
							&request_id,
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
					JobStatus::Running => {
						log::debug!("request_id: {request_id} - Waiting to update status of `{build_step_job_name}`");
						tokio::time::sleep(Duration::from_millis(1000)).await; // 1 secs
						service::queue_create_ci_build_step(
							build_step,
							config,
							&request_id,
						)
						.await?;
					}
				}
			} else {
				let db_status_for_this_step = db::get_build_step_status(
					connection,
					&build_step.id.build_id.repo_id,
					build_step.id.build_id.build_num,
					build_step.id.step_id,
				)
				.await?
				.unwrap_or(BuildStepStatus::Errored);

				if db_status_for_this_step == BuildStepStatus::Running {
					// job is missing in k8s, so mark as errored
					log::info!(
						"request_id: {} - Job is missing in k8s, marking step `{}` as errored",
						request_id,
						build_step_job_name
					);
					service::delete_kubernetes_job(
						&build_namespace,
						&build_step_job_name,
						kubeconfig.clone(),
						&request_id,
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
						log::info!(
							"request_id: {} - Build step `{}` skipped as dependencies errored out",
							request_id,
							build_step_job_name
						);
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
					BuildStepStatus::Cancelled => {
						log::info!("request_id: {request_id} - Build step `{build_step_job_name}` stopped");
						db::update_build_step_status(
							connection,
							&build_step.id.build_id.repo_id,
							build_step.id.build_id.build_num,
							build_step.id.step_id,
							BuildStepStatus::Cancelled,
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
						let runner_resource =
							db::get_runner_resource_for_build(
								connection,
								&build_step.id.build_id.repo_id,
								build_step.id.build_id.build_num,
							)
							.await?
							.status(500)?;

						service::create_ci_job_in_kubernetes(
							&build_namespace,
							&build_step,
							runner_resource.ram_in_mb(),
							runner_resource.cpu_in_milli(),
							kubeconfig,
							config,
							&request_id,
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
						tokio::time::sleep(Duration::from_millis(1000)).await; // 1 secs
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
					service::delete_kubernetes_namespace(
						&build_id.get_build_namespace(),
						db::get_all_default_regions(&mut *connection)
							.await?
							.into_iter()
							.find_map(|region| {
								if region.status == RegionStatus::Active {
									region
										.config_file
										.map(|Json(config)| config)
								} else {
									None
								}
							})
							.status(500)?,
						&request_id,
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
					service::update_github_commit_status_for_build(
						connection,
						&build_id.repo_id,
						build_id.build_num,
						CommitStatus::Failed,
					)
					.await?;
				}
				BuildStepStatus::Succeeded => {
					log::info!(
						"request_id: {request_id} - Build `{build_id}` succeed"
					);
					service::delete_kubernetes_namespace(
						&build_id.get_build_namespace(),
						db::get_all_default_regions(&mut *connection)
							.await?
							.into_iter()
							.find_map(|region| {
								if region.status == RegionStatus::Active {
									region
										.config_file
										.map(|Json(config)| config)
								} else {
									None
								}
							})
							.status(500)?,
						&request_id,
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
					service::update_github_commit_status_for_build(
						connection,
						&build_id.repo_id,
						build_id.build_num,
						CommitStatus::Success,
					)
					.await?;
				}
				BuildStepStatus::Running | BuildStepStatus::WaitingToStart => {
					log::debug!("request_id: {request_id} - Waiting to clean `{build_id}`");
					tokio::time::sleep(Duration::from_millis(1500)).await; // 1 secs
					service::queue_clean_ci_build_pipeline(
						build_id,
						config,
						&request_id,
					)
					.await?;
				}
				BuildStepStatus::Cancelled => {
					log::debug!("request_id: {request_id} - Cleaning stopped build `{build_id}`");
					service::delete_kubernetes_namespace(
						&build_id.get_build_namespace(),
						db::get_all_default_regions(&mut *connection)
							.await?
							.into_iter()
							.find_map(|region| {
								if region.status == RegionStatus::Active {
									region
										.config_file
										.map(|Json(config)| config)
								} else {
									None
								}
							})
							.status(500)?,
						&request_id,
					)
					.await?;
				}
			}
		}
		CIData::SyncRepo {
			workspace_id,
			git_provider_id,
			request_id,
			github_access_token,
		} => {
			service::sync_github_repos(
				connection,
				&workspace_id,
				&git_provider_id,
				github_access_token,
				&request_id,
			)
			.await?;
			db::set_syncing(
				connection,
				&git_provider_id,
				false,
				Some(Utc::now()),
			)
			.await?;
		}
	}
	Ok(())
}
