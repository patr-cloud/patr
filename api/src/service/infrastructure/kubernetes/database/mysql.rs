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
			EnvVar,
			EnvVarSource,
			ExecAction,
			PersistentVolumeClaim,
			PersistentVolumeClaimSpec,
			PersistentVolumeClaimVolumeSource,
			PodSpec,
			PodTemplateSpec,
			Probe,
			ResourceRequirements,
			Secret,
			SecretKeySelector,
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
	api::{DeleteParams, ListParams, Patch, PatchParams},
	config::Kubeconfig,
	core::ObjectMeta,
	Api,
};

use crate::{
	models::managed_database::StatefulSetConfig,
	service::ext_traits::DeleteOpt,
	utils::Error,
};

pub async fn patch_kubernetes_mysql_database(
	workspace_id: &Uuid,
	database_id: &Uuid,
	db_pwd: impl Into<String>,
	db_plan: &DatabasePlanType,
	kubeconfig: Kubeconfig,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client =
		super::super::get_kubernetes_client(kubeconfig).await?;

	// names
	let namespace = workspace_id.as_str();
	let statefulset_config = get_sts_config(database_id);

	// constants
	let secret_key_for_db_pwd = "password";
	let mysql_port = 3306;

	let labels =
		BTreeMap::from([("database".to_owned(), database_id.to_string())]);

	log::trace!("request_id: {request_id} - Creating secret for database pwd");

	let secret_spec_for_db_pwd = Secret {
		metadata: ObjectMeta {
			name: Some(statefulset_config.secret_name_for_db_pwd.clone()),
			..Default::default()
		},
		type_: Some("kubernetes.io/basic-auth".to_owned()),
		string_data: Some(
			[(secret_key_for_db_pwd.to_owned(), db_pwd.into())].into(),
		),
		..Default::default()
	};

	Api::<Secret>::namespaced(kubernetes_client.clone(), namespace)
		.patch(
			&statefulset_config.secret_name_for_db_pwd,
			&PatchParams::apply(&statefulset_config.secret_name_for_db_pwd),
			&Patch::Apply(secret_spec_for_db_pwd),
		)
		.await?;

	log::trace!("request_id: {request_id} - Creating service for database");

	let service_for_db = Service {
		metadata: ObjectMeta {
			name: Some(statefulset_config.svc_name_for_db.to_owned()),
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
			&statefulset_config.svc_name_for_db,
			&PatchParams::apply(&statefulset_config.svc_name_for_db),
			&Patch::Apply(service_for_db),
		)
		.await?;

	log::trace!("request_id: {request_id} - Creating statefulset for database");

	let db_pvc_template = PersistentVolumeClaim {
		metadata: ObjectMeta {
			name: Some(statefulset_config.pvc_claim_for_db.to_owned()),
			..Default::default()
		},
		spec: Some(PersistentVolumeClaimSpec {
			access_modes: Some(vec!["ReadWriteOnce".to_owned()]),
			resources: Some(ResourceRequirements {
				requests: Some(
					[(
						"storage".to_owned(),
						Quantity(db_plan.volume.to_string()),
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
				image: Some("mysql:8.0".to_owned()),
				env: Some(vec![EnvVar {
					name: "MYSQL_ROOT_PASSWORD".to_owned(),
					value_from: Some(EnvVarSource {
						secret_key_ref: Some(SecretKeySelector {
							name: Some(
								statefulset_config.secret_name_for_db_pwd,
							),
							key: secret_key_for_db_pwd.to_owned(),
							..Default::default()
						}),
						..Default::default()
					}),
					..Default::default()
				}]),
				ports: Some(vec![ContainerPort {
					name: Some("mysql".to_owned()),
					container_port: mysql_port,
					..Default::default()
				}]),
				volume_mounts: Some(vec![VolumeMount {
					name: statefulset_config.pvc_claim_for_db.to_owned(),
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
								Quantity(db_plan.memory_count.to_string()),
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
							"mysqladmin ping -p$MYSQL_ROOT_PASSWORD".to_owned(),
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
							"mysql -h 127.0.0.1 -u root -p$MYSQL_ROOT_PASSWORD -e \"SELECT 1\"".to_owned()
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
				name: statefulset_config.pvc_claim_for_db.to_owned(),
				persistent_volume_claim: Some(
					PersistentVolumeClaimVolumeSource {
						claim_name: statefulset_config
							.pvc_claim_for_db
							.to_owned(),
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
			name: Some(statefulset_config.sts_name_for_db.clone()),
			..Default::default()
		},
		spec: Some(StatefulSetSpec {
			service_name: statefulset_config.svc_name_for_db,
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
			&statefulset_config.sts_name_for_db,
			&PatchParams::apply(&statefulset_config.sts_name_for_db),
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
		super::super::get_kubernetes_client(kubeconfig).await?;

	// names
	let namespace = workspace_id.as_str();
	let statefulset_config = get_sts_config(database_id);

	let label = format!("database={}", database_id);

	log::trace!("request_id: {request_id} - Deleting statefulset for database");
	Api::<StatefulSet>::namespaced(kubernetes_client.clone(), namespace)
		.delete_opt(
			&statefulset_config.sts_name_for_db,
			&DeleteParams::default(),
		)
		.await?;

	log::trace!("request_id: {request_id} - Deleting service for database");
	Api::<Service>::namespaced(kubernetes_client.clone(), namespace)
		.delete_opt(
			&statefulset_config.svc_name_for_db,
			&DeleteParams::default(),
		)
		.await?;

	log::trace!("request_id: {request_id} - Deleting secret for database");
	Api::<Secret>::namespaced(kubernetes_client.clone(), namespace)
		.delete_opt(
			&statefulset_config.secret_name_for_db_pwd,
			&DeleteParams::default(),
		)
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

pub fn get_sts_config(database_id: &Uuid) -> StatefulSetConfig {
	StatefulSetConfig {
		secret_name_for_db_pwd: format!("db-pwd-{database_id}"),
		svc_name_for_db: format!("service-{database_id}"),
		sts_name_for_db: format!("db-{database_id}"),
		pvc_claim_for_db: format!("db-pvc-{database_id}"),
	}
}
