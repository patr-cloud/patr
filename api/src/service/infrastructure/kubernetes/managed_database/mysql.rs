use api_models::{
	models::workspace::infrastructure::database::ManagedDatabasePlan,
	utils::Uuid,
};
use eve_rs::AsError;
use kube::{
	core::{ApiResource, DynamicObject},
	Api,
};

use crate::{
	service::{self, infrastructure::kubernetes::get_kubernetes_config},
	utils::{settings::Settings, Error},
};

pub async fn create_mysql_database_cluster(
	config: &Settings,
	workspace_id: &Uuid,
	cluster_name: &str,
	db_root_username: &str,
	db_root_password: &str,
	_db_name: &str, // todo : create a db schema in this name afte init mysql
	database_id: &Uuid,
	num_nodes: u64,
	database_plan: &ManagedDatabasePlan,
	request_id: &Uuid,
) -> Result<(), Error> {
	let namespace = workspace_id.as_str();
	log::trace!("request_id: {request_id} - Creating mysql cluster in k8s with name `{cluster_name}` under namespace `{namespace}`");

	// todo : use something else for k8s db naming like base32
	if cluster_name.len() > 22 ||
		!cluster_name
			.chars()
			.all(|ch| ch.is_ascii_lowercase() || ch.is_digit(10))
	{
		return Error::as_result().status(400).body(
			"invalid cluster name : should contain lowercase alphanumerics and length should be less than 22"
		)?;
	}

	service::queue_create_mysql_database(
		config,
		request_id.to_owned(),
		workspace_id.to_owned(),
		database_id.to_owned(),
		cluster_name.to_owned(),
		db_root_username.to_owned(),
		db_root_password.to_owned(),
		num_nodes as i32,
		database_plan.to_owned(),
	)
	.await?;

	Ok(())
}

pub async fn delete_mysql_database_cluster(
	config: &Settings,
	request_id: &Uuid,
	workspace_id: &Uuid,
	database_id: &Uuid,
	cluster_name: &str,
	num_nodes: i32,
) -> Result<(), Error> {
	let kube_client = get_kubernetes_config(config).await?;
	let namespace = workspace_id.as_str();
	log::trace!("request_id: {request_id} - Deleting mysql cluster in k8s with name `{cluster_name}`");

	let cluster_resource = ApiResource {
		group: "mysql.oracle.com".to_string(),
		version: "mysql.oracle.com/v2".to_string(),
		api_version: "mysql.oracle.com/v2".to_string(),
		kind: "InnoDBCluster".to_string(),
		plural: "innodbclusters".to_string(),
	};

	// check whether given db is available
	let db_custer_api = Api::<DynamicObject>::namespaced_with(
		kube_client.clone(),
		namespace,
		&cluster_resource,
	);
	if db_custer_api
		.get_opt(&format!("mysql-{}", cluster_name))
		.await?
		.is_none()
	{
		return Err(Error::empty().status(500).body("resource doesn't exist"));
	}

	service::queue_delete_mysql_database(
		config,
		request_id.to_owned(),
		workspace_id.to_owned(),
		database_id.to_owned(),
		cluster_name.to_owned(),
		num_nodes,
	)
	.await?;

	Ok(())
}
