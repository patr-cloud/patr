use std::time::Duration;

use api_models::models::workspace::infrastructure::database::ManagedDatabaseStatus;
use chrono::Utc;
use tokio::time;

use crate::{
	db,
	models::rabbitmq::DatabaseRequestData,
	service,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn process_request(
	connection: &mut <Database as sqlx::Database>::Connection,
	request_data: DatabaseRequestData,
	config: &Settings,
) -> Result<(), Error> {
	match request_data {
		DatabaseRequestData::CheckAndUpdateStatus {
			workspace_id,
			database_id,
			request_id,
			password,
		} => {
			let database = db::get_managed_database_by_id_including_deleted(
				connection,
				&database_id,
			)
			.await?;

			let database =
				match database {
					Some(database) => database,
					None => {
						log::info!("expected database id {database_id} not present in db");
						return Ok(());
					}
				};

			if database.status != ManagedDatabaseStatus::Creating {
				log::info!("Database {database_id} is not in creating state. Hence stopping status check message");
				return Ok(());
			}

			let start_time = Utc::now();

			let kubeconfig = service::get_kubernetes_config_for_region(
				connection,
				&database.region,
			)
			.await?
			.0;

			log::trace!("Checking patr database status: {database_id}");
			loop {
				let status = service::get_kubernetes_database_status(
					&workspace_id,
					&database_id,
					kubeconfig.clone(),
					&request_id,
				)
				.await?;

				if status != ManagedDatabaseStatus::Creating {
					db::update_managed_database_status(
						connection,
						&database_id,
						&status,
					)
					.await?;

					if status == ManagedDatabaseStatus::Running {
						log::trace!(
							"Setting root password for database: {database_id}"
						);
						time::sleep(Duration::from_secs(15)).await;
						service::change_database_password(
							connection,
							&database_id,
							&request_id,
							&password,
							config,
						)
						.await?;
					}
					return Ok(());
				}
				time::sleep(Duration::from_millis(1000)).await;

				if Utc::now() - start_time > chrono::Duration::seconds(30) {
					db::update_managed_database_status(
						connection,
						&database_id,
						&ManagedDatabaseStatus::Errored,
					)
					.await?;
					break;
				}
			}

			// requeue it again
			Err(Error::empty())
		}
		DatabaseRequestData::ChangeMongoPasswordAndStatus {
			workspace_id,
			database_id,
			request_id,
			password,
		} => {
			let database =
				db::get_managed_database_by_id(connection, &database_id)
					.await?;

			let database =
				match database {
					Some(database) => database,
					None => {
						log::info!("expected database id {database_id} not present in db");
						return Ok(());
					}
				};

			if database.status != ManagedDatabaseStatus::Creating {
				log::info!("Database {database_id} is not in creating state. Hence stopping status check message");
				return Ok(());
			}

			let kubeconfig = service::get_kubernetes_config_for_region(
				connection,
				&database.region,
			)
			.await?
			.0;

			let start_time = Utc::now();

			log::trace!(
				"Check patr database status to update password: {database_id}"
			);
			loop {
				let status = service::get_kubernetes_database_status(
					&workspace_id,
					&database_id,
					kubeconfig.clone(),
					&request_id,
				)
				.await?;

				if status == ManagedDatabaseStatus::Running {
					log::trace!(
						"Changing password for database: {database_id}"
					);

					time::sleep(Duration::from_secs(15)).await;

					service::change_mongo_database_password(
						&workspace_id,
						kubeconfig.clone(),
						&request_id,
						&database_id,
						&password,
					)
					.await?;

					log::trace!("request_id: {request_id} - Changing Mongo statefulset config to enable auth");
					let database_plan = db::get_database_plan_by_id(
						connection,
						&database.database_plan_id,
					)
					.await?;

					service::patch_kubernetes_mongo_database(
						&database.workspace_id,
						&database.id,
						&database_plan,
						kubeconfig.clone(),
						&request_id,
						true,
						false,
					)
					.await?;

					break;
				}
				time::sleep(Duration::from_millis(1000)).await;

				if Utc::now() - start_time > chrono::Duration::seconds(30) {
					log::trace!("request_id: {request_id} - Password change failed for Mongo. Pod not in running state");
					db::update_managed_database_status(
						connection,
						&database_id,
						&ManagedDatabaseStatus::Errored,
					)
					.await?;
					// requeue it again
					return Err(Error::empty());
				}
			}

			// Password changed. Mongo is updated. Now update the status
			loop {
				let status = service::get_kubernetes_database_status(
					&workspace_id,
					&database_id,
					kubeconfig.clone(),
					&request_id,
				)
				.await?;

				if status != ManagedDatabaseStatus::Creating {
					db::update_managed_database_status(
						connection,
						&database_id,
						&status,
					)
					.await?;

					return Ok(());
				}
				time::sleep(Duration::from_millis(1000)).await;

				if Utc::now() - start_time > chrono::Duration::seconds(30) {
					db::update_managed_database_status(
						connection,
						&database_id,
						&ManagedDatabaseStatus::Errored,
					)
					.await?;
					// requeue it again
					return Err(Error::empty());
				}
			}
		}
	}
}
