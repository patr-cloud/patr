use std::collections::BTreeMap;

use api_models::{
	models::workspace::infrastructure::database::PatrDatabasePlan,
	utils::Uuid,
};
use k8s_openapi::{
	api::{
		apps::v1::{StatefulSet, StatefulSetSpec},
		core::v1::{
			Container,
			ContainerPort,
			EnvVar,
			EnvVarSource,
			ExecAction,
			PersistentVolumeClaim,
			PersistentVolumeClaimSpec,
			PodSpec,
			PodTemplateSpec,
			Probe,
			ResourceRequirements,
			Secret,
			SecretKeySelector,
			Service,
			ServicePort,
			ServiceSpec,
			VolumeMount,
		},
	},
	apimachinery::pkg::{
		api::resource::Quantity,
		apis::meta::v1::LabelSelector,
	},
};
use kube::{
	api::{DeleteParams, ListParams, Patch, PatchParams},
	config::Kubeconfig,
	core::ObjectMeta,
	Api,
};

use crate::{
	service::{ext_traits::DeleteOpt, ResourceLimitsForPlan},
	utils::Error,
};

pub async fn create_kubernetes_mysql_database(
	workspace_id: &Uuid,
	database_id: &Uuid,
	db_pwd: impl Into<String>,
	db_plan: &PatrDatabasePlan,
	kubeconfig: Kubeconfig,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client =
		super::super::get_kubernetes_client(kubeconfig.auth_details).await?;

	// names
	let namespace = workspace_id.as_str();
	let sec_name_for_db_pwd = format!("db-pwd-{database_id}");
	let svc_name_for_db = format!("db-{database_id}");
	let sts_name_for_db = format!("db-{database_id}");
	let pvc_prefix_for_db = "pvc"; // actual name will be `pvc-{sts_name_for_db}-{sts_ordinal}`

	// constants
	let sec_key_for_db_pwd = "password";
	let mysql_port = 3306;

	// plan
	let (db_ram, db_cpu, db_volume) = db_plan.get_resource_limits();

	let labels =
		BTreeMap::from([("database".to_owned(), database_id.to_string())]);

	log::trace!("request_id: {request_id} - Creating secret for database pwd");

	let secret_spec_for_db_pwd = Secret {
		metadata: ObjectMeta {
			name: Some(sec_name_for_db_pwd.clone()),
			..Default::default()
		},
		string_data: Some(
			[(sec_key_for_db_pwd.to_owned(), db_pwd.into())].into(),
		),
		..Default::default()
	};

	Api::<Secret>::namespaced(kubernetes_client.clone(), namespace)
		.patch(
			&sec_name_for_db_pwd,
			&PatchParams::apply(&sec_name_for_db_pwd),
			&Patch::Apply(secret_spec_for_db_pwd),
		)
		.await?;

	log::trace!("request_id: {request_id} - Creating service for database");

	let service_for_db = Service {
		metadata: ObjectMeta {
			name: Some(svc_name_for_db.to_owned()),
			labels: Some(labels.clone()),
			..Default::default()
		},
		spec: Some(ServiceSpec {
			cluster_ip: Some("None".to_owned()),
			selector: Some(labels.clone()),
			ports: Some(vec![ServicePort {
				port: mysql_port,
				..Default::default()
			}]),
			..Default::default()
		}),
		..Default::default()
	};

	Api::<Service>::namespaced(kubernetes_client.clone(), namespace)
		.patch(
			&svc_name_for_db,
			&PatchParams::apply(&svc_name_for_db),
			&Patch::Apply(service_for_db),
		)
		.await?;

	log::trace!("request_id: {request_id} - Creating statefulset for database");

	let db_pvc_template = PersistentVolumeClaim {
		metadata: ObjectMeta {
			name: Some(pvc_prefix_for_db.to_owned()),
			labels: Some(labels.clone()),
			..Default::default()
		},
		spec: Some(PersistentVolumeClaimSpec {
			// todo: for patr region default storage class will take care,
			// but for byoc user needs to set up a default storage class
			access_modes: Some(vec!["ReadWriteOnce".to_owned()]),
			resources: Some(ResourceRequirements {
				requests: Some([("storage".to_owned(), db_volume)].into()),
				..Default::default()
			}),
			..Default::default()
		}),
		..Default::default()
	};

	let db_pod_template = PodTemplateSpec {
		metadata: Some(ObjectMeta {
			labels: Some(labels.clone()),
			..Default::default()
		}),
		spec: Some(PodSpec {
			containers: vec![Container {
				name: "mysql".to_owned(),
				image: Some("mysql:8.0.31-debian".to_owned()),
				image_pull_policy: Some("Always".to_string()),
				env: Some(vec![EnvVar {
					name: "MYSQL_ROOT_PASSWORD".to_owned(),
					value_from: Some(EnvVarSource {
						secret_key_ref: Some(SecretKeySelector {
							name: Some(sec_name_for_db_pwd),
							key: sec_key_for_db_pwd.to_owned(),
							..Default::default()
						}),
						..Default::default()
					}),
					..Default::default()
				}]),
				ports: Some(vec![ContainerPort {
					container_port: mysql_port,
					..Default::default()
				}]),
				volume_mounts: Some(vec![VolumeMount {
					name: pvc_prefix_for_db.to_owned(),
					mount_path: "/var/lib/mysql".to_owned(),
					sub_path: Some("mysql".to_owned()),
					..Default::default()
				}]),
				resources: Some(ResourceRequirements {
					// https://blog.kubecost.com/blog/requests-and-limits/#the-tradeoffs
					// using too low values for resource request will
					// result in frequent pod restarts if memory usage
					// increases and may result in starvation
					//
					// currently used 5% of the mininum deployment
					// machine type as a request values
					requests: Some(
						[
							("memory".to_string(), Quantity("25M".to_owned())),
							("cpu".to_string(), Quantity("50m".to_owned())),
						]
						.into(),
					),
					limits: Some(
						[
							("memory".to_string(), db_ram),
							("cpu".to_string(), db_cpu),
						]
						.into(),
					),
				}),
				// todo: validate readiness probe config
				// https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle/#when-should-you-use-a-startup-probe
				readiness_probe: Some(Probe {
					exec: Some(ExecAction {
						command: Some(
							["mysql", "-h", "127.0.0.1", "-e", "SELECT 1"]
								.into_iter()
								.map(ToOwned::to_owned)
								.collect(),
						),
					}),
					initial_delay_seconds: Some(10),
					failure_threshold: Some(10),
					period_seconds: Some(10),
					timeout_seconds: Some(2),
					..Default::default()
				}),
				..Default::default()
			}],
			..Default::default()
		}),
	};

	let statefulset_spec_for_db = StatefulSet {
		metadata: ObjectMeta {
			name: Some(sts_name_for_db.clone()),
			labels: Some(labels.clone()),
			..Default::default()
		},
		spec: Some(StatefulSetSpec {
			service_name: svc_name_for_db,
			replicas: Some(1),
			selector: LabelSelector {
				match_labels: Some(labels.clone()),
				..Default::default()
			},
			volume_claim_templates: Some(vec![db_pvc_template]),
			template: db_pod_template,
			..Default::default()
		}),
		..Default::default()
	};

	Api::<StatefulSet>::namespaced(kubernetes_client, namespace)
		.patch(
			&sts_name_for_db,
			&PatchParams::apply(&sts_name_for_db),
			&Patch::Apply(statefulset_spec_for_db),
		)
		.await?;

	Ok(())
}

pub async fn delete_kubernetes_mysql_database(
	workspace_id: &Uuid,
	database_id: &Uuid,
	kubeconfig: Kubeconfig,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client =
		super::super::get_kubernetes_client(kubeconfig.auth_details).await?;

	// names
	let namespace = workspace_id.as_str();
	let sec_name_for_db_pwd = format!("db-pwd-{database_id}");
	let svc_name_for_db = format!("db-{database_id}");
	let sts_name_for_db = format!("db-{database_id}");

	let label = format!("database={}", database_id);

	log::trace!("request_id: {request_id} - Deleting statefulset for database");
	Api::<StatefulSet>::namespaced(kubernetes_client.clone(), namespace)
		.delete_opt(&sts_name_for_db, &DeleteParams::default())
		.await?;

	log::trace!("request_id: {request_id} - Deleting service for database");
	Api::<Service>::namespaced(kubernetes_client.clone(), namespace)
		.delete_opt(&svc_name_for_db, &DeleteParams::default())
		.await?;

	log::trace!("request_id: {request_id} - Deleting secret for database");
	Api::<Secret>::namespaced(kubernetes_client.clone(), namespace)
		.delete_opt(&sec_name_for_db_pwd, &DeleteParams::default())
		.await?;

	log::trace!("request_id: {request_id} - Deleting volume for database");
	// pvc don't get automatically deleted, whenever sts is deleted.
	// so manually delete the pvc based on label.
	let pvcs = Api::<PersistentVolumeClaim>::namespaced(
		kubernetes_client.clone(),
		namespace,
	)
	.list(&ListParams::default().labels(&label))
	.await?
	.into_iter()
	.filter_map(|pvc| pvc.metadata.name);

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
