use std::time::Duration;

use api_models::{
	models::workspace::infrastructure::deployment::{
		Deployment,
		DeploymentRunningDetails,
		DeploymentStatus,
	},
	utils::Uuid,
};
use chrono::Utc;
use tokio::time;

use crate::{
	db,
	models::{
		rabbitmq::DeploymentRequestData,
		rbac::{self, permissions},
	},
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
		DeploymentRequestData::Update {
			workspace_id,
			deployment,
			image_name,
			digest,
			running_details,
			user_id,
			login_id,
			ip_address,
			metadata,
			request_id,
		} => {
			let audit_log_id =
				db::generate_new_workspace_audit_log_id(connection).await?;

			db::create_workspace_audit_log(
				connection,
				&audit_log_id,
				&workspace_id,
				&ip_address,
				&Utc::now(),
				Some(&user_id),
				Some(&login_id),
				&deployment.id,
				rbac::PERMISSIONS
					.get()
					.unwrap()
					.get(permissions::workspace::infrastructure::deployment::EDIT)
					.unwrap(),
				&request_id,
				&serde_json::to_value(metadata)?,
				false,
				true,
			)
			.await?;

			update_deployment_and_db_status(
				connection,
				&workspace_id,
				&deployment,
				&image_name,
				digest.as_deref(),
				&running_details,
				config,
				&request_id,
			)
			.await
		}
	}
}

pub async fn update_deployment_and_db_status(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	deployment: &Deployment,
	image_name: &str,
	digest: Option<&str>,
	running_details: &DeploymentRunningDetails,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let result = service::update_kubernetes_deployment(
		workspace_id,
		deployment,
		image_name,
		digest,
		running_details,
		config,
		request_id,
	)
	.await;

	if let Err(err) = result {
		log::error!(
			"request_id: {} - Error occured while deploying `{}`: {}",
			request_id,
			deployment.id,
			err.get_error()
		);
		// TODO log in audit log that there was an error while deploying
		db::update_deployment_status(
			connection,
			&deployment.id,
			&DeploymentStatus::Errored,
		)
		.await?;

		Err(err)
	} else {
		let start_time = Utc::now();

		loop {
			let status = service::get_kubernetes_deployment_status(
				connection,
				&deployment.id,
				workspace_id.as_str(),
				config,
			)
			.await?;

			if status != DeploymentStatus::Deploying {
				// TODO Log in audit log about the updated status
				db::update_deployment_status(
					connection,
					&deployment.id,
					&status,
				)
				.await?;
				break;
			}
			time::sleep(Duration::from_millis(500)).await;

			if Utc::now() - start_time > chrono::Duration::seconds(30) {
				break;
			}
		}

		Ok(())
	}
}
