use std::time::Duration;

use api_models::{
	models::workspace::infrastructure::deployment::{
		Deployment,
		DeploymentRunningDetails,
		DeploymentStatus,
	},
	utils::Uuid,
};
use tokio::time;

use crate::{
	db,
	models::{rabbitmq::DeploymentRequestData, rbac::permissions},
	service,
	utils::{get_current_time_millis, settings::Settings, Error},
	Database,
};

pub(super) async fn process_request(
	connection: &mut <Database as sqlx::Database>::Connection,
	request_data: DeploymentRequestData,
	config: &Settings,
) -> Result<(), Error> {
	match request_data {
		DeploymentRequestData::Create {
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

			let audit_log_id = db::generate_new_workspace_audit_log_id(
				context.get_database_connection(),
			)
			.await?;

			let metadata = serde_json::to_value(DeploymentMetadata::Create {
				deployment: Deployment {
					id: id.clone(),
					name: name.to_string(),
					registry: registry.clone(),
					image_tag: image_tag.to_string(),
					status: DeploymentStatus::Created,
					region: region.clone(),
					machine_type: machine_type.clone(),
				},
				running_details: deployment_running_details.clone(),
			})?;

			db::create_workspace_audit_log(
				context.get_database_connection(),
				&audit_log_id,
				&workspace_id,
				&ip_address,
				Utc::now(),
				Some(&user_id),
				Some(&login_id),
				&id,
				rbac::PERMISSIONS
					.get()
					.unwrap()
					.get(permissions::workspace::infrastructure::deployment::EDIT)
					.unwrap(),
				&request_id,
				&metadata,
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
		DeploymentRequestData::UpdateImage {
			workspace_id,
			deployment,
			image_name,
			digest,
			running_details,
			request_id,
		} => {
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
		DeploymentRequestData::Start {
			workspace_id,
			deployment,
			image_name,
			digest,
			running_details,
			request_id,
		} => {
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
		DeploymentRequestData::Stop {
			workspace_id,
			deployment_id,
			request_id,
		} => {
			service::delete_kubernetes_deployment(
				&workspace_id,
				&deployment_id,
				config,
				&request_id,
			)
			.await
		}
		DeploymentRequestData::Update {
			workspace_id,
			deployment,
			image_name,
			digest,
			running_details,
			request_id,
		} => {
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
		DeploymentRequestData::Delete {
			workspace_id,
			deployment_id,
			request_id,
		} => {
			service::delete_kubernetes_deployment(
				&workspace_id,
				&deployment_id,
				config,
				&request_id,
			)
			.await
		}
	}
}

async fn update_deployment_and_db_status(
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
		let start_time = get_current_time_millis();

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

			if get_current_time_millis() - start_time > 30000 {
				break;
			}
		}

		Ok(())
	}
}
