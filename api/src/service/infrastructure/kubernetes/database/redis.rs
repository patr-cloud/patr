use std::collections::BTreeMap;

use api_models::{
	models::workspace::infrastructure::database::DatabasePlanType,
	utils::Uuid,
};
use k8s_openapi::{
	api::{
		apps::v1::{StatefulSet, StatefulSetSpec},
		core::v1::{
			ConfigMap,
			ConfigMapVolumeSource,
			Container,
			ContainerPort,
			EnvVar,
			ExecAction,
			KeyToPath,
			PersistentVolumeClaim,
			PersistentVolumeClaimSpec,
			PersistentVolumeClaimVolumeSource,
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
		util::intstr::IntOrString,
	},
};
use kube::{
	api::{Patch, PatchParams},
	config::Kubeconfig,
	core::ObjectMeta,
	Api,
};

use crate::{
	service::{
		get_database_pvc_name,
		get_database_service_name,
		get_database_sts_name,
		infrastructure::kubernetes::get_kubernetes_client,
	},
	utils::Error,
};

pub async fn patch_kubernetes_redis_database(
	workspace_id: &Uuid,
	database_id: &Uuid,
	db_pwd: &String,
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
	let configmap_name_for_db = get_database_config_name(database_id);

	// constants
	let redis_port = 6379;
	let redis_version = "redis:5.0.4";

	let labels =
		BTreeMap::from([("database".to_owned(), database_id.to_string())]);

	log::trace!("request_id: {request_id} - Creating configmap for database");

	let mut config_data = BTreeMap::new();
	config_data.insert(
		"redis-config".to_owned(),
		vec![
			format!("requirepass {}", db_pwd.to_owned()),
			"save 60 1".to_owned(),
			"dir /redis-master-data".to_owned(),
		]
		.join("\n"),
	);

	let config_for_db = ConfigMap {
		metadata: ObjectMeta {
			name: Some(configmap_name_for_db.to_owned()),
			..Default::default()
		},
		data: Some(config_data.clone()),
		..Default::default()
	};

	Api::<ConfigMap>::namespaced(kubernetes_client.clone(), namespace)
		.patch(
			&configmap_name_for_db,
			&PatchParams::apply(&configmap_name_for_db),
			&Patch::Apply(config_for_db),
		)
		.await?;

	log::trace!("request_id: {request_id} - Creating service for database");

	let service_for_db = Service {
		metadata: ObjectMeta {
			name: Some(svc_name_for_db.to_owned()),
			..Default::default()
		},
		spec: Some(ServiceSpec {
			cluster_ip: Some("None".to_owned()),
			selector: Some(labels.clone()),
			ports: Some(vec![ServicePort {
				name: Some("redis".to_owned()),
				port: redis_port,
				target_port: Some(IntOrString::Int(redis_port)),
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
				name: "redis".to_owned(),
				image: Some(redis_version.to_owned()),
				command: Some(vec!["redis-server".to_owned()]),
				args: Some(vec!["/redis-master/redis.conf".to_owned()]),
				ports: Some(vec![ContainerPort {
					container_port: redis_port,
					..Default::default()
				}]),
				env: Some(vec![EnvVar {
					name: "MASTER".to_owned(),
					value: Some("true".to_owned()),
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
				liveness_probe: Some(Probe {
					exec: Some(ExecAction {
						command: Some(vec![
							"redis-cli".to_owned(),
							"ping".to_owned(),
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
							"redis-cli".to_owned(),
							"ping".to_owned(),
						]),
					}),
					initial_delay_seconds: Some(5),
					failure_threshold: Some(10),
					period_seconds: Some(2),
					timeout_seconds: Some(5),
					..Default::default()
				}),
				volume_mounts: Some(vec![
					VolumeMount {
						name: pvc_claim_for_db.to_owned(),
						mount_path: "/redis-master-data".to_owned(),
						..Default::default()
					},
					VolumeMount {
						name: "config".to_owned(),
						mount_path: "/redis-master".to_owned(),
						..Default::default()
					},
				]),
				..Default::default()
			}],
			volumes: Some(vec![
				Volume {
					name: pvc_claim_for_db.to_owned(),
					persistent_volume_claim: Some(
						PersistentVolumeClaimVolumeSource {
							claim_name: pvc_claim_for_db,
							..Default::default()
						},
					),
					..Default::default()
				},
				Volume {
					name: "config".to_owned(),
					config_map: Some(ConfigMapVolumeSource {
						name: Some(configmap_name_for_db.to_owned()),
						items: Some(vec![KeyToPath {
							key: "redis-config".to_owned(),
							path: "redis.conf".to_owned(),
							..Default::default()
						}]),
						..Default::default()
					}),
					..Default::default()
				},
			]),
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

pub fn get_database_config_name(database_id: &Uuid) -> String {
	format!("config-{database_id}")
}

pub async fn change_redis_database_password(
	workspace_id: &Uuid,
	database_id: &Uuid,
	kubeconfig: Kubeconfig,
	request_id: &Uuid,
	new_password: &String,
) -> Result<(), Error> {
	log::trace!("request_id: {request_id} - Editing redis config map and changing password");

	let sts_name_for_db = get_database_sts_name(database_id);
	let configmap_name_for_db = get_database_config_name(database_id);
	let namespace = workspace_id.as_str();
	let kubernetes_client = get_kubernetes_client(kubeconfig).await?;

	let mut config_data = BTreeMap::new();
	config_data.insert(
		"redis-config".to_owned(),
		vec![
			format!("requirepass {}", new_password.to_owned()),
			"save 60 1".to_owned(),
			"dir /redis-master-data".to_owned(),
		]
		.join("\n"),
	);

	let config_for_db = ConfigMap {
		metadata: ObjectMeta {
			name: Some(configmap_name_for_db.to_owned()),
			..Default::default()
		},
		data: Some(config_data.clone()),
		..Default::default()
	};

	Api::<ConfigMap>::namespaced(kubernetes_client.clone(), namespace)
		.patch(
			&configmap_name_for_db,
			&PatchParams::apply(&configmap_name_for_db),
			&Patch::Apply(config_for_db),
		)
		.await?;

	// Statefulset does not restart automatically on configmap change
	// Trigger manual restart
	Api::<StatefulSet>::namespaced(kubernetes_client.clone(), namespace)
		.restart(&sts_name_for_db)
		.await?;

	log::trace!("request_id: {request_id} - Password changed successfully");
	Ok(())
}
