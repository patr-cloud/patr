use std::{collections::BTreeMap, fmt::Display, time::Duration};

use api_models::{
	models::{
		ci::file_format::EnvVarValue,
		workspace::ci::git_provider::{BuildStatus, BuildStepStatus},
	},
	utils::Uuid,
};
use chrono::Utc;
use eve_rs::AsError;
use serde::{Deserialize, Serialize};

use crate::{
	db,
	models::rabbitmq::CIData,
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
		CIData::BuildStep {
			build_step,
			request_id,
		} => {
			let build_namespace = build_step.id.build_id.get_build_namespace();
			let build_step_job_name = build_step.id.get_job_name();

			let step_status = service::get_ci_job_status_in_kubernetes(
				&build_namespace,
				&build_step_job_name,
				config,
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
						config,
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
							config,
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
							config,
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
						config,
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
						let build_machine_type =
							db::get_build_machine_type_for_repo(
								connection,
								&build_step.id.build_id.repo_id,
							)
							.await?
							.status(500)?;

						service::create_ci_job_in_kubernetes(
							&build_namespace,
							&build_step,
							build_machine_type.ram as u32,
							build_machine_type.cpu as u32,
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
		CIData::CancelBuild {
			build_id,
			request_id,
		} => {
			log::info!("request_id: {request_id} - Build `{build_id}` stopped");
			db::update_build_status(
				&mut *connection,
				&build_id.repo_id,
				build_id.build_num,
				BuildStatus::Cancelled,
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
					service::delete_kubernetes_namespace(
						&build_id.get_build_namespace(),
						service::get_kubernetes_config_for_default_region(
							config,
						),
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
				}
				BuildStepStatus::Succeeded => {
					log::info!(
						"request_id: {request_id} - Build `{build_id}` succeed"
					);
					service::delete_kubernetes_namespace(
						&build_id.get_build_namespace(),
						service::get_kubernetes_config_for_default_region(
							config,
						),
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
						service::get_kubernetes_config_for_default_region(
							config,
						),
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
			db::set_syncing(connection, &git_provider_id, false, Some(Utc::now())).await?;
		}
	}
	Ok(())
}
