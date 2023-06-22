mod mysql;
mod psql;
mod redis;

use api_models::{
	models::workspace::infrastructure::database::ManagedDatabaseStatus,
	utils::Uuid,
};
use k8s_openapi::api::{
	apps::v1::StatefulSet,
	core::v1::{ConfigMap, PersistentVolumeClaim, Service},
};
use kube::{
	api::{DeleteParams, ListParams},
	config::Kubeconfig,
	Api,
};

pub use self::{mysql::*, psql::*, redis::*};
use crate::{
	service::{
		ext_traits::DeleteOpt,
		infrastructure::kubernetes::get_kubernetes_client,
	},
	utils::Error,
};

pub async fn get_kubernetes_database_status(
	workspace_id: &Uuid,
	database_id: &Uuid,
	kubeconfig: Kubeconfig,
	request_id: &Uuid,
) -> Result<ManagedDatabaseStatus, Error> {
	let kubernetes_client = super::get_kubernetes_client(kubeconfig).await?;

	// names
	let namespace = workspace_id.as_str();
	let sts_name_for_db = get_database_sts_name(database_id);

	log::trace!("request_id: {request_id} - Getting statefulset status for database {database_id}");
	let sts = Api::<StatefulSet>::namespaced(kubernetes_client, namespace)
		.get_opt(&sts_name_for_db)
		.await?;

	let ready_replicas = match sts
		.and_then(|sts| sts.status)
		.and_then(|status| status.available_replicas)
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

pub async fn delete_kubernetes_database(
	workspace_id: &Uuid,
	database_id: &Uuid,
	kubeconfig: Kubeconfig,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client = get_kubernetes_client(kubeconfig).await?;

	// names
	let namespace = workspace_id.as_str();
	let sts_name_for_db = get_database_sts_name(database_id);
	let svc_name_for_db = get_database_service_name(database_id);
	let configmap_name_for_db = get_database_config_name(database_id);

	let label = format!("database={}", database_id);

	log::trace!("request_id: {request_id} - Deleting statefulset for database");
	Api::<StatefulSet>::namespaced(kubernetes_client.clone(), namespace)
		.delete_opt(&sts_name_for_db, &DeleteParams::default())
		.await?;

	log::trace!("request_id: {request_id} - Deleting service for database");
	Api::<Service>::namespaced(kubernetes_client.clone(), namespace)
		.delete_opt(&svc_name_for_db, &DeleteParams::default())
		.await?;

	log::trace!("request_id: {request_id} - Deleting configmap for database");
	Api::<ConfigMap>::namespaced(kubernetes_client.clone(), namespace)
		.delete_opt(&configmap_name_for_db, &DeleteParams::default())
		.await?;

	log::trace!("request_id: {request_id} - Deleting volume for database");

	// Manually deleting PVC as it does not get deleted automatically
	// PVC related to the database is first found using labels(database =
	// database_id)
	let pvcs = Api::<PersistentVolumeClaim>::namespaced(
		kubernetes_client.clone(),
		namespace,
	)
	.list(&ListParams::default().labels(&label))
	.await?
	.into_iter()
	.filter_map(|pvc| pvc.metadata.name);

	// Then the PVC is deleted one-by-one
	for pvc in pvcs {
		Api::<PersistentVolumeClaim>::namespaced(
			kubernetes_client.clone(),
			namespace,
		)
		.delete_opt(&pvc, &DeleteParams::default())
		.await?;
	}

	Ok(())
}

pub fn get_database_sts_name(database_id: &Uuid) -> String {
	format!("db-{database_id}")
}

pub fn get_database_service_name(database_id: &Uuid) -> String {
	format!("service-{database_id}")
}

pub fn get_database_pvc_name(database_id: &Uuid) -> String {
	format!("db-pvc-{database_id}")
}
