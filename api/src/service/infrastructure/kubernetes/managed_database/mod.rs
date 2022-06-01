mod mysql;

use api_models::{
	models::workspace::infrastructure::database::{
		ManagedDatabaseEngine,
		ManagedDatabasePlan,
	},
	utils::Uuid,
};
use eve_rs::AsError;
use rand::{distributions::Alphanumeric, thread_rng, Rng};

use crate::utils::{settings::Settings, Error};

pub async fn create_managed_database_cluster(
	config: &Settings,
	request_id: &Uuid,
	workspace_id: &Uuid,
	database_id: &Uuid,
	cluster_name: &str,
	db_name: &str,
	engine: &ManagedDatabaseEngine,
	_version: &str, // todo
	num_nodes: u64,
	database_plan: &ManagedDatabasePlan,
	_region: &str, // todo
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - Creating a managed database on k8s with id: {}",
		request_id,
		database_id,
	);

	let username = "patr_admin".to_string();
	let password = thread_rng()
		.sample_iter(&Alphanumeric)
		.take(8)
		.map(char::from)
		.collect::<String>();

	log::trace!(
		"request_id: {} - sending the create db cluster request to k8s",
		request_id
	);
	match engine {
		ManagedDatabaseEngine::Postgres => {
			return Error::as_result()
				.status(400)
				.body("Currently postgres db is not supported")?;
		}
		ManagedDatabaseEngine::Mysql => {
			mysql::create_mysql_database_cluster(
				config,
				workspace_id,
				cluster_name,
				&username,
				&password,
				db_name,
				database_id,
				num_nodes,
				database_plan,
				request_id,
			)
			.await?
		}
	}

	log::trace!("request_id: {} - database created", request_id);
	Ok(())
}

pub async fn delete_database(
	config: &Settings,
	request_id: &Uuid,
	workspace_id: &Uuid,
	database_id: &Uuid,
	cluster_name: &str,
	num_nodes: i32,
	engine: &ManagedDatabaseEngine,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - Deleting managed database on k8s with id: {}",
		request_id,
		database_id,
	);

	match engine {
		ManagedDatabaseEngine::Postgres => {
			return Error::as_result()
				.status(400)
				.body("Currently postgres db is not supported")?;
		}
		ManagedDatabaseEngine::Mysql => {
			mysql::delete_mysql_database_cluster(
				config,
				request_id,
				workspace_id,
				database_id,
				cluster_name,
				num_nodes,
			)
			.await?
		}
	}

	Ok(())
}
