mod mysql;
mod psql;

use api_models::{
	models::workspace::infrastructure::database::{
		PatrDatabasePlan,
		PatrDatabaseStatus,
	},
	utils::Uuid,
};
use k8s_openapi::{
	api::apps::v1::StatefulSet,
	apimachinery::pkg::api::resource::Quantity,
};
use kube::Api;
pub use mysql::*;
pub use psql::*;

use crate::{service::KubernetesConfigDetails, utils::Error};

pub trait ResourceLimitsForPlan {
	fn get_resource_limits(&self) -> (Quantity, Quantity, Quantity);
}

impl ResourceLimitsForPlan for PatrDatabasePlan {
	fn get_resource_limits(&self) -> (Quantity, Quantity, Quantity) {
		let (ram, cpu, volume) = match self {
			PatrDatabasePlan::db_1r_1c_10v => ("1Gi", "1", "10Gi"),
			PatrDatabasePlan::db_2r_2c_25v => ("2Gi", "2", "25Gi"),
		};

		(
			Quantity(ram.to_owned()),
			Quantity(cpu.to_owned()),
			Quantity(volume.to_owned()),
		)
	}
}

pub async fn get_kubernetes_database_status(
	workspace_id: &Uuid,
	database_id: &Uuid,
	kubeconfig: KubernetesConfigDetails,
	request_id: &Uuid,
) -> Result<PatrDatabaseStatus, Error> {
	let kubernetes_client =
		super::get_kubernetes_client(kubeconfig.auth_details).await?;

	// names
	let namespace = workspace_id.as_str();
	let sts_name_for_db = format!("db-{database_id}");

	log::trace!("request_id: {request_id} - Getting statefulset status for database {database_id}");
	let sts =
		Api::<StatefulSet>::namespaced(kubernetes_client.clone(), namespace)
			.get_opt(&sts_name_for_db)
			.await?;

	let ready_replicas = match sts
		.and_then(|sts| sts.status)
		.and_then(|status| status.ready_replicas)
	{
		Some(ready_replicas) => ready_replicas,
		None => return Ok(PatrDatabaseStatus::Errored),
	};

	// todo: need to change when database cluster is used
	if ready_replicas == 1 {
		Ok(PatrDatabaseStatus::Running)
	} else {
		Ok(PatrDatabaseStatus::Creating)
	}
}
