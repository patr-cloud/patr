mod mysql;

use api_models::{
	models::workspace::infrastructure::database::ManagedDatabaseStatus,
	utils::Uuid,
};
use k8s_openapi::api::apps::v1::StatefulSet;
use kube::{config::Kubeconfig, Api};
pub use mysql::*;

use crate::utils::Error;

pub async fn get_kubernetes_database_status(
	workspace_id: &Uuid,
	database_id: &Uuid,
	kubeconfig: Kubeconfig,
	request_id: &Uuid,
) -> Result<ManagedDatabaseStatus, Error> {
	let kubernetes_client = super::get_kubernetes_client(kubeconfig).await?;

	// names
	let namespace = workspace_id.as_str();
	let sts_name_for_db = format!("db-{database_id}");

	log::trace!("request_id: {request_id} - Getting statefulset statis for database {database_id}");
	let sts =
		Api::<StatefulSet>::namespaced(kubernetes_client.clone(), namespace)
			.get_opt(&sts_name_for_db)
			.await?;

	let ready_replicas = match sts
		.and_then(|sts| sts.status)
		.and_then(|status| status.ready_replicas)
	{
		Some(ready_replicas) => ready_replicas,
		None => return Ok(ManagedDatabaseStatus::Errored),
	};

	// todo: need to change when database cluster is used
	if ready_replicas == 1 {
		Ok(ManagedDatabaseStatus::Running)
	} else {
		Ok(ManagedDatabaseStatus::Creating)
	}
}
