use std::time::Duration;

use api_models::utils::Uuid;
use eve_rs::AsError;
use k8s_openapi::api::{batch::v1::Job, core::v1::PersistentVolumeClaim};
use kube::Api;
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BuildStepId {
	pub workspace_id: Uuid,
	pub repo_id: Uuid,
	pub build_num: i64,
	pub step_id: i32,
}

pub async fn process_request(
	connection: &mut <Database as sqlx::Database>::Connection,
	request_data: CIData,
	config: &Settings,
) -> Result<(), Error> {
	let kube_client = service::get_kubernetes_config(config).await?;

	match request_data {
		CIData::InitRepo {
			build_step_id,
			job,
			request_id,
		} => {
			log::debug!(
				"request_id: {request_id} - Initializing clone repo job"
			);

			let result = Api::<Job>::namespaced(kube_client, "patrci") // TODO
				.create(&Default::default(), &job)
				.await;

			match result {
				Ok(_) => {
					log::debug!(
						"request_id: {request_id} - Clone repo job creation success, queuing job to look for status update"
					);
					db::update_build_step_status(
						connection,
						&build_step_id.repo_id,
						build_step_id.build_num,
						build_step_id.step_id,
						"running",
					)
					.await?;
					service::queue_update_build_step_status(
						build_step_id,
						config,
						&request_id,
					)
					.await?;
				}
				Err(err) => {
					log::error!(
						"request_id: {} - Error while creating clone repo job, updating error status in db : {}", request_id, err
					);
					db::update_build_step_status(
						connection,
						&build_step_id.repo_id,
						build_step_id.build_num,
						build_step_id.step_id,
						"errored",
					)
					.await?;
				}
			}
		}
		CIData::CreateBuildStep {
			build_step_id:
				BuildStepId {
					workspace_id,
					repo_id,
					build_num,
					step_id,
				},
			job,
			request_id,
		} => {
			log::debug!(
				"request_id: {request_id} - Checking to create job for ci-{repo_id}-{build_num}-{step_id}"
			);

			// TODO: step_id
			let previous_status = db::get_build_step_status(
				connection,
				&repo_id,
				build_num,
				step_id - 1,
			)
			.await?;
			if previous_status.eq_ignore_ascii_case("errored") ||
				previous_status.eq_ignore_ascii_case("skipped-parent_error")
			{
				log::debug!(
					"request_id: {request_id} - Updating status as skipped-parent_error for ci-{repo_id}-{build_num}-{step_id}"
				);
				db::update_build_step_status(
					connection,
					&repo_id,
					build_num,
					step_id,
					"skipped-parent_error",
				)
				.await?;
			} else if previous_status.eq_ignore_ascii_case("waiting_to_start") ||
				previous_status.eq_ignore_ascii_case("running")
			{
				// wait until parent job completes, so requeue this job after
				// some time by returing an error
				tokio::time::sleep(Duration::from_secs(60)).await;
				return Error::as_result()
					.status(200)
					.body("waiting to create build step")?;
			} else if previous_status.eq_ignore_ascii_case("success") {
				// previous state is success, so we can spinup this job now
				let result = Api::<Job>::namespaced(kube_client, "patrci") // TODO
					.create(&Default::default(), &job)
					.await;

				match result {
					Ok(_) => {
						log::debug!(
						"request_id: {request_id} - Creating job for ci-{repo_id}-{build_num}-{step_id} success"
					);
						db::update_build_step_status(
							connection, &repo_id, build_num, step_id, "running",
						)
						.await?;
						service::queue_update_build_step_status(
							BuildStepId {
								workspace_id,
								repo_id,
								build_num,
								step_id,
							},
							config,
							&request_id,
						)
						.await?;
					}
					Err(err) => {
						log::error!(
						"request_id: {} - Error while creating job for ci-{}-{}-{}, updating error status in db : {}", request_id, repo_id, build_num, step_id, err
					);
						db::update_build_step_status(
							connection, &repo_id, build_num, step_id, "errored",
						)
						.await?;
					}
				}
			} else {
				// TODO: handle invalid state
			}
		}
		CIData::UpdateBuildStepStatus {
			build_step_id:
				BuildStepId {
					workspace_id: _,
					repo_id,
					build_num,
					step_id,
				},
			request_id,
		} => {
			log::debug!(
				"request_id: {request_id} - Checking the status for ci-{repo_id}-{build_num}-{step_id}"
			);

			let result =
				Api::<Job>::namespaced(kube_client, "patrci") // TODO
					.get_status(&format!("ci-{repo_id}-{build_num}-{step_id}"))
					.await;

			let mut is_errored = false;
			match result {
				Ok(job) => {
					let status = job.status.map(|status| {
						(
							status.active.unwrap_or_default(),
							status.succeeded.unwrap_or_default(),
							status.failed.unwrap_or_default(),
						)
					});
					if let Some((active, succeeded, failed)) = status {
						// one pod per job is used, so it is safe to check this
						// way
						if failed == 1 {
							log::error!(
								"request_id: {request_id} - Error while getting status of job from JobStatus, updating error status in db"
							);
							is_errored = true;
						} else if succeeded == 1 {
							log::info!(
								"request_id: {request_id} - Step completed successfully, updating in db"
							);
							db::update_build_step_status(
								connection, &repo_id, build_num, step_id,
								"success",
							)
							.await?;
						} else if active == 1 {
							// currently running, so requeue this job after some
							// time by returing an error
							tokio::time::sleep(Duration::from_secs(60)).await;
							return Error::as_result()
								.status(200)
								.body("waiting for updating build status")?;
						} else {
							// TODO: handle invalid state
						}
					} else {
						log::error!(
							"request_id: {request_id} - Error while getting status of job from JobStatus, updating error status in db"
						);
						is_errored = true;
					}
				}
				Err(err) => {
					log::error!(
						"request_id: {} - Error while getting status of job from k8s, updating error status in db : {}", request_id, err
					);
					is_errored = true;
				}
			}

			if is_errored {
				db::update_build_step_status(
					connection, &repo_id, build_num, step_id, "errored",
				)
				.await?;
			}
		}
		CIData::CleanBuild {
			build_id:
				BuildId {
					workspace_id: _,
					repo_id,
					build_num,
				},
			request_id: _,
		} => {
			let steps = db::get_build_steps_for_build(
				&mut *connection,
				&repo_id,
				build_num,
			)
			.await?;
			let status = steps.last().map(|step| step.step_status.clone());

			match status {
				Some(status)
					if status.eq_ignore_ascii_case("waiting_to_start") ||
						status.eq_ignore_ascii_case("running") =>
				{
					// currently running, so requeue this job after some time by
					// returing an error
					tokio::time::sleep(Duration::from_secs(120)).await;
					return Err(Error::empty());
				}
				Some(status)
					if status.eq_ignore_ascii_case("errored") ||
						status.eq_ignore_ascii_case(
							"skipped-parent_error",
						) =>
				{
					db::update_build_status(
						&mut *connection,
						&repo_id,
						build_num,
						"errored",
					)
					.await?;
				}
				_ => {
					db::update_build_status(
						&mut *connection,
						&repo_id,
						build_num,
						"success",
					)
					.await?;
				}
			}

			// delete all jobs associated with this pvc
			// since ttl is 120 for jobs sleep for 150 sec and then delete pvc
			tokio::time::sleep(Duration::from_secs(150)).await;

			// remove pvc
			let pvc_name = format!("ci-{repo_id}-{build_num}");
			Api::<PersistentVolumeClaim>::namespaced(kube_client, "patrci")
				.delete(&pvc_name, &Default::default())
				.await?;
		}
	}
	Ok(())
}
