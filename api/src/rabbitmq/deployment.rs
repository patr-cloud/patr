use std::time::Duration;

use api_models::models::workspace::infrastructure::deployment::DeploymentStatus;
use chrono::Utc;
use tokio::time;

use crate::{
	db,
	models::rabbitmq::DeploymentRequestData,
	service,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn process_request(
	connection: &mut <Database as sqlx::Database>::Connection,
	request_data: DeploymentRequestData,
	config: &Settings,
) -> Result<(), Error> {
	match request_data {
		DeploymentRequestData::CheckAndUpdateStatus {
			workspace_id,
			deployment_id,
		} => {
			let deployment = db::get_deployment_by_id_including_deleted(
				connection,
				&deployment_id,
			)
			.await?;

			let deployment = match deployment {
				Some(deployment) => deployment,
				None => {
					log::info!("expected deployment id {deployment_id} not present in db");
					return Ok(());
				}
			};

			if deployment.status != DeploymentStatus::Deploying {
				log::info!("Deployment {deployment_id} is not in deploying state. Hence stopping status check message");
				return Ok(());
			}

			let start_time = Utc::now();

			loop {
				let status = service::get_kubernetes_deployment_status(
					connection,
					&deployment_id,
					workspace_id.as_str(),
					config,
				)
				.await?;

				if status != DeploymentStatus::Deploying {
					// TODO Log in audit log about the updated status
					db::update_deployment_status(
						connection,
						&deployment_id,
						&status,
					)
					.await?;
					return Ok(());
				}
				time::sleep(Duration::from_millis(500)).await;

				if Utc::now() - start_time > chrono::Duration::seconds(30) {
					break;
				}
			}

			time::sleep(Duration::from_secs(2)).await;

			// requeue it again
			Err(Error::empty())
		}
		DeploymentRequestData::UpdateImage {
			workspace_id,
			deployment,
			image_name,
			digest,
			running_details,
			request_id,
		} => {
			db::update_deployment_status(
				connection,
				&deployment.id,
				&DeploymentStatus::Deploying,
			)
			.await?;

			let result = service::update_deployment_image(
				connection,
				&workspace_id,
				&deployment.id,
				&deployment.name,
				&deployment.registry,
				&digest,
				&deployment.image_tag,
				&image_name,
				&deployment.region,
				&deployment.machine_type,
				&running_details,
				config,
				&request_id,
			)
			.await;

			if let Err(err) = result {
				log::error!(
					"request_id: {} - Error occured while updating deployment `{}` to image `{}` : {}",
					request_id,
					deployment.id,
					digest,
					err.get_error()
				);
				// TODO log in audit log that there was an error while deploying
				db::update_deployment_status(
					connection,
					&deployment.id,
					&DeploymentStatus::Errored,
				)
				.await?;

				return Ok(());
			}

			service::queue_check_and_update_deployment_status(
				&workspace_id,
				&deployment.id,
				config,
				&request_id,
			)
			.await?;

			Ok(())
		}
	}
}
