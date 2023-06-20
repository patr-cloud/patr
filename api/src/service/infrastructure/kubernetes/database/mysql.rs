use std::collections::BTreeMap;

use api_models::{
	models::workspace::infrastructure::database::DatabasePlanType,
	utils::Uuid,
};
use k8s_openapi::{
	api::{
		apps::v1::{StatefulSet, StatefulSetSpec},
		core::v1::{
			Container,
			ContainerPort,
			ExecAction,
			PersistentVolumeClaim,
			PersistentVolumeClaimSpec,
			PersistentVolumeClaimVolumeSource,
			Pod,
			PodSpec,
			PodTemplateSpec,
			Probe,
			ResourceRequirements,
			Service,
			ServicePort,
			ServiceSpec,
			Volume,
			VolumeMount,
		},
	},
	apimachinery::pkg::{
		api::resource::Quantity,
		apis::meta::v1::LabelSelector,
	},
};
use kube::{
	api::{AttachParams, DeleteParams, ListParams, Patch, PatchParams},
	config::Kubeconfig,
	core::ObjectMeta,
	Api,
};

use crate::{
	service::{
		ext_traits::DeleteOpt,
		get_database_sts_name,
		infrastructure::kubernetes::get_kubernetes_client,
	},
	utils::Error,
};

pub async fn patch_kubernetes_mysql_database(
	workspace_id: &Uuid,
	database_id: &Uuid,
	db_plan: &DatabasePlanType,
	kubeconfig: Kubeconfig,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client = get_kubernetes_client(kubeconfig).await?;

	// names
	let namespace = workspace_id.as_str();
	let sts_name_for_db = get_database_sts_name(database_id);
	let svc_name_for_db = get_database_service_name(database_id);
	let pvc_claim_for_db = get_database_pvc_name(database_id);

	// constants
	let mysql_port = 3306;
	let mysql_version = "patrcloud/mysql:8.0";

	let labels =
		BTreeMap::from([("database".to_owned(), database_id.to_string())]);

	log::trace!("request_id: {request_id} - Creating service for database");

	let service_for_db = Service {
		metadata: ObjectMeta {
			name: Some(svc_name_for_db.to_owned()),
			labels: Some(labels.clone()),
			..Default::default()
		},
		spec: Some(ServiceSpec {
			selector: Some(labels.clone()),
			ports: Some(vec![ServicePort {
				name: Some("mysql".to_owned()),
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
			name: Some(pvc_claim_for_db.to_owned()),
			..Default::default()
		},
		spec: Some(PersistentVolumeClaimSpec {
			access_modes: Some(vec!["ReadWriteOnce".to_owned()]),
			resources: Some(ResourceRequirements {
				requests: Some(
					[(
						"storage".to_owned(),
						Quantity(format! {"{0}Gi", db_plan.volume}),
					)]
					.into(),
				),
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
				image: Some(mysql_version.to_owned()),
				image_pull_policy: Some("Always".to_owned()),
				ports: Some(vec![ContainerPort {
					name: Some("mysql".to_owned()),
					container_port: mysql_port,
					..Default::default()
				}]),
				volume_mounts: Some(vec![VolumeMount {
					name: pvc_claim_for_db.to_owned(),
					mount_path: "/var/lib/mysql".to_owned(),
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
							(
								"memory".to_string(),
								Quantity(
									format! {"{0}Gi", db_plan.memory_count},
								),
							),
							(
								"cpu".to_string(),
								Quantity(db_plan.cpu_count.to_string()),
							),
						]
						.into(),
					),
				}),
				// https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle/#when-should-you-use-a-startup-probe
				// startup probe are required when the containers take a long
				// time to spin up according to docs it is recommended to use a
				// startup probe instead of liveness probe in such instances
				liveness_probe: Some(Probe {
					exec: Some(ExecAction {
						command: Some(vec![
							"bash".to_owned(),
							"-c".to_owned(),
							vec!["mysqladmin ping".to_owned()].join("\n"),
						]),
					}),
					initial_delay_seconds: Some(30),
					period_seconds: Some(10),
					timeout_seconds: Some(5),
					..Default::default()
				}),
				readiness_probe: Some(Probe {
					exec: Some(ExecAction {
						command: Some(vec![
							"bash".to_owned(),
							"-c".to_owned(),
							vec!["mysql -u root -e \"SELECT 1\"".to_owned()]
								.join("\n"),
						]),
					}),
					initial_delay_seconds: Some(5),
					failure_threshold: Some(10),
					period_seconds: Some(2),
					timeout_seconds: Some(1),
					..Default::default()
				}),
				..Default::default()
			}],
			volumes: Some(vec![Volume {
				name: pvc_claim_for_db.to_owned(),
				persistent_volume_claim: Some(
					PersistentVolumeClaimVolumeSource {
						claim_name: pvc_claim_for_db.to_owned(),
						..Default::default()
					},
				),
				..Default::default()
			}]),
			..Default::default()
		}),
	};

	let statefulset_spec_for_db = StatefulSet {
		metadata: ObjectMeta {
			name: Some(sts_name_for_db.clone()),
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
	let kubernetes_client = get_kubernetes_client(kubeconfig).await?;

	// names
	let namespace = workspace_id.as_str();
	let sts_name_for_db = get_database_sts_name(database_id);
	let svc_name_for_db = get_database_service_name(database_id);

	let label = format!("database={}", database_id);

	log::trace!("request_id: {request_id} - Deleting statefulset for database");
	Api::<StatefulSet>::namespaced(kubernetes_client.clone(), namespace)
		.delete_opt(&sts_name_for_db, &DeleteParams::default())
		.await?;

	log::trace!("request_id: {request_id} - Deleting service for database");
	Api::<Service>::namespaced(kubernetes_client.clone(), namespace)
		.delete_opt(&svc_name_for_db, &DeleteParams::default())
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

pub async fn change_mysql_database_password(
	workspace_id: &Uuid,
	database_id: &Uuid,
	kubeconfig: Kubeconfig,
	request_id: &Uuid,
	new_password: &String,
) -> Result<(), Error> {
	log::trace!("request_id: {request_id} - Connecting to MySQL server and changing password");

	let sts_name_for_db = get_database_sts_name(database_id);
	let namespace = workspace_id.as_str();
	let kubernetes_client = get_kubernetes_client(kubeconfig).await?;

	Api::<Pod>::namespaced(kubernetes_client.clone(), namespace)
		.exec(
			&format!("{sts_name_for_db}-0"),
			[
				"bash".to_owned(),
				"-c".to_owned(),
				format!("mysql -e \"ALTER USER 'root'@'%' IDENTIFIED BY '{new_password}'; FLUSH PRIVILEGES;\"")
			],
			&AttachParams {
				..Default::default()
			},
		)
		.await?;

	log::trace!("request_id: {request_id} - Password changed successfully");
	Ok(())
}

pub fn get_database_service_name(database_id: &Uuid) -> String {
	format!("service-{database_id}")
}

pub fn get_database_pvc_name(database_id: &Uuid) -> String {
	format!("db-pvc-{database_id}")
}
