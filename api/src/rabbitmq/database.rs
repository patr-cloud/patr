use std::time::Duration;

use api_models::{
	models::workspace::infrastructure::database::{
		ManagedDatabasePlan,
		ManagedDatabaseStatus,
	},
	utils::Uuid,
};
use k8s_openapi::api::core::v1::{PersistentVolumeClaim, Secret};
use kube::{
	api::{DeleteParams, PostParams},
	core::{ApiResource, DynamicObject, ObjectMeta, TypeMeta},
	Api,
};
use serde_json::json;
use tokio::time;

use crate::{
	db::{
		update_managed_database_credentials_for_database,
		update_managed_database_status,
	},
	models::rabbitmq::DatabaseRequestData,
	service::get_kubernetes_config,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn process_request(
	connection: &mut <Database as sqlx::Database>::Connection,
	request_data: DatabaseRequestData,
	config: &Settings,
) -> Result<(), Error> {
	match request_data {
		DatabaseRequestData::CreateMySQL {
			request_id,
			workspace_id,
			database_id,
			cluster_name,
			db_root_username,
			db_root_password,
			num_nodes,
			database_plan,
		} => {
			create_mysql_database_cluster_from_rabbit_mq(
				&workspace_id,
				&cluster_name,
				&db_root_username,
				&db_root_password,
				&database_id,
				num_nodes,
				&database_plan,
				&request_id,
				config,
				connection,
			)
			.await
		}
		DatabaseRequestData::DeleteMySQL {
			request_id,
			workspace_id,
			database_id,
			cluster_name,
			num_nodes,
		} => {
			delete_mysql_database_cluster_from_rabbit_mq(
				&request_id,
				&workspace_id,
				&database_id,
				&cluster_name,
				num_nodes,
				config,
				connection,
			)
			.await
		}
	}
}

async fn create_mysql_database_cluster_from_rabbit_mq(
	workspace_id: &Uuid,
	cluster_name: &str,
	db_root_username: &str,
	db_root_password: &str,
	database_id: &Uuid,
	num_nodes: i32,
	database_plan: &ManagedDatabasePlan,
	request_id: &Uuid,
	config: &Settings,
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), Error> {
	let namespace = workspace_id.as_str();
	let kube_client = get_kubernetes_config(config).await?;

	log::trace!("request_id: {request_id} - Creating secret for mysql cluster with database_id `{database_id}`");
	// store username and password for db in secrets
	Api::<Secret>::namespaced(kube_client.clone(), namespace)
		.create(
			&PostParams::default(),
			&serde_json::from_value(json!({
			  "apiVersion": "v1",
			  "kind": "Secret",
			  "metadata": {
				"name": format!("mysql-pwd-{}", database_id)
			  },
			  "stringData": {
				"rootUser": db_root_username,
				"rootHost": "%",
				"rootPassword": db_root_password
			  }
			}))
			.expect("json parsing should not fail"),
		)
		.await?;

	// create mysql cluster with the above credentials
	let cluster_resource = ApiResource {
		group: "mysql.oracle.com".to_string(),
		version: "mysql.oracle.com/v2".to_string(),
		api_version: "mysql.oracle.com/v2".to_string(),
		kind: "InnoDBCluster".to_string(),
		plural: "innodbclusters".to_string(),
	};

	let storage_size_in_gb = match database_plan {
		ManagedDatabasePlan::Nano => 1,
		ManagedDatabasePlan::Micro => 2,
		ManagedDatabasePlan::Small => 4,
		ManagedDatabasePlan::Medium => 8,
		ManagedDatabasePlan::Large => 16,
		ManagedDatabasePlan::Xlarge => 32,
		ManagedDatabasePlan::Xxlarge => 32,
		ManagedDatabasePlan::Mammoth => 32,
	};

	let db_custer_spec = json!({
	  "spec": {
		"secretName": format!("mysql-pwd-{}", database_id),
		"tlsUseSelfSigned": true,
		"instances": num_nodes, // todo
		"router": {
		  "instances": 1u8, // todo
		},
		"datadirVolumeClaimTemplate": {
		  "accessModes": [
			"ReadWriteOnce"
		  ],
		  "resources": {
			"requests": {
			  "storage": format!("{storage_size_in_gb}Gi")
			}
		  }
		}
	  }
	});

	let cluster_object = DynamicObject {
		types: Some(TypeMeta {
			api_version: "mysql.oracle.com/v2".to_string(),
			kind: "InnoDBCluster".to_string(),
		}),
		metadata: ObjectMeta {
			name: Some(format!("mysql-{}", cluster_name)),
			..ObjectMeta::default()
		},
		data: db_custer_spec,
	};

	let db_custer_api = Api::<DynamicObject>::namespaced_with(
		kube_client,
		namespace,
		&cluster_resource,
	);

	log::trace!("request_id: {request_id} - Creating innodb cluster for mysql with database_id `{database_id}`");
	db_custer_api
		.create(&PostParams::default(), &cluster_object)
		.await?;

	// wait before checking status
	time::sleep(Duration::from_millis(1000)).await;

	// wait for mysql cluster to spin up
	let mut is_errored = false;
	wait_for_mysql_to_spin_up(
		db_custer_api,
		&format!("mysql-{}", cluster_name),
	)
	.await
	.map_err(|err| {
		is_errored = true;
		log::error!(
			"Error while creating managed database, {}",
			err.get_error()
		);
		err
	})?;

	let _ = update_managed_database_status(
		connection,
		database_id,
		if is_errored {
			&ManagedDatabaseStatus::Errored
		} else {
			&ManagedDatabaseStatus::Running
		},
	)
	.await;

	// todo : create db schema
	// todo : update connection string - how to allow external access

	// update credentials and connection str
	let _ = update_managed_database_credentials_for_database(
		connection,
		database_id,
		&format!("mysql-{cluster_name}.{namespace}.svc.cluster.local"),
		0,
		db_root_username,
		db_root_password,
	)
	.await;
	log::info!("request_id: {request_id} - Created mysql database {cluster_name} is running");

	Ok(())
}

async fn wait_for_mysql_to_spin_up(
	db_custer_api: Api<DynamicObject>,
	innodb_cluster_name: &str,
) -> Result<(), Error> {
	loop {
		let response = db_custer_api.get_status(innodb_cluster_name).await?;
		let status = response
			.data
			.get("status")
			.and_then(|obj| obj.get("cluster"))
			.and_then(|obj| obj.get("status"))
			.and_then(|obj| obj.as_str())
			.expect("status metadata should be present");

		// todo: handle creation failure and choke-up issues 
		// maybe use some timer like 10 mins or use rabbitmq as statemachie of db status
		if status == "ONLINE" {
			return Ok(());
		}

		time::sleep(Duration::from_millis(1000)).await;
	}
}

async fn delete_mysql_database_cluster_from_rabbit_mq(
	request_id: &Uuid,
	workspace_id: &Uuid,
	database_id: &Uuid,
	cluster_name: &str,
	num_nodes: i32,
	config: &Settings,
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), Error> {
	log::info!("request_id: {request_id} - Deleting mysql database {cluster_name} from workspace {workspace_id}");

	let namespace = workspace_id.as_str();
	let kube_client = get_kubernetes_config(config).await?;

	let db_custer_api = Api::<DynamicObject>::namespaced_with(
		kube_client.clone(),
		namespace,
		&ApiResource {
			group: "mysql.oracle.com".to_string(),
			version: "mysql.oracle.com/v2".to_string(),
			api_version: "mysql.oracle.com/v2".to_string(),
			kind: "InnoDBCluster".to_string(),
			plural: "innodbclusters".to_string(),
		},
	);

	// 1. remove the db from k8s
	db_custer_api
		.delete(&format!("mysql-{}", cluster_name), &DeleteParams::default())
		.await?;

	// wait until db is deleted
	time::sleep(Duration::from_millis(1000)).await;

	// 2. remove the secret from k8s
	Api::<Secret>::namespaced(kube_client.clone(), namespace)
		.delete(
			&format!("mysql-pwd-{}", database_id),
			&DeleteParams::default(),
		)
		.await?;

	// 3. remove the pvc from k8s
	let pvc_api =
		Api::<PersistentVolumeClaim>::namespaced(kube_client, namespace);
	for i in 0..num_nodes {
		pvc_api
			.delete(
				&format!("datadir-mysql-{cluster_name}-{i}"),
				&DeleteParams::default(),
			)
			.await?;
	}

	// 4. update the db status as deleted
	update_managed_database_status(
		connection,
		database_id,
		&ManagedDatabaseStatus::Deleted,
	)
	.await?;

	log::info!("request_id: {request_id} - Deleted mysql database {cluster_name} from workspace {workspace_id}");

	Ok(())
}
