use std::time::Duration;

use api_models::models::workspace::infrastructure::database::PatrDatabaseStatus;
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
		} => {
			let database = db::get_patr_database_by_id_including_deleted(
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

			if database.status != PatrDatabaseStatus::Creating {
				log::info!("Database {database_id} is not in creating state. Hence stopping status check message");
				return Ok(());
			}

			let start_time = Utc::now();

			loop {
				let status = service::get_patr_database_status(
					connection,
					&workspace_id,
					&database_id,
					config,
					&request_id,
				)
				.await?;

				if status != PatrDatabaseStatus::Creating {
					db::update_patr_database_status(
						connection,
						&database_id,
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
	}
}
