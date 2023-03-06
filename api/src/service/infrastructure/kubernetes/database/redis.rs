use std::collections::BTreeMap;

use api_models::{
	models::workspace::infrastructure::database::PatrDatabasePlan,
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
			EmptyDirVolumeSource,
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
			TCPSocketAction,
			Volume,
			VolumeMount,
		},
	},
	apimachinery::pkg::{
		api::resource::Quantity,
		apis::meta::v1::LabelSelector,
		util::intstr::IntOrString,
	},
	ByteString,
};
use kube::{
	api::{DeleteParams, ListParams, Patch, PatchParams},
	core::ObjectMeta,
	Api,
};

use crate::{
	service::{
		ext_traits::DeleteOpt,
		KubernetesConfigDetails,
		ResourceLimitsForPlan,
	},
	utils::Error,
};

pub async fn create_kubernetes_redis_database(
	workspace_id: &Uuid,
	database_id: &Uuid,
	db_pwd: &String,
	db_plan: &PatrDatabasePlan,
	kubeconfig: KubernetesConfigDetails,
	request_id: &Uuid,
	replica_numbers: i32,
) -> Result<(), Error> {
	let kubernetes_client =
		super::super::get_kubernetes_client(kubeconfig.auth_details).await?;

	// names
	let namespace = workspace_id.as_str();
	let secret_name_for_db_pwd = format!("db-pwd-{database_id}");
	let svc_name_for_db = format!("db-{database_id}-service");
	let sts_name_for_db = format!("db-{database_id}-sts");
	let sts_port_name_for_db = format!("db-{database_id}-port");
	let pvc_prefix_for_db = "data";
	let configmap_name_for_db = format!("db-{database_id}-config");

	// constants
	let secret_key_for_db_pwd = "password";
	let redis_port = 6379;

	// plan
	let (db_ram, db_cpu, db_volume) = db_plan.get_resource_limits();
	let labels =
		BTreeMap::from([("database".to_owned(), database_id.to_string())]);

	log::trace!("request_id: {request_id} - Creating secret for database pwd");

	let mut secret_data: BTreeMap<String, ByteString> = BTreeMap::new();
	secret_data.insert(secret_key_for_db_pwd.to_owned(), db_pwd);

	let secret_spec_for_db_pwd = Secret {
		metadata: ObjectMeta {
			name: Some(secret_name_for_db_pwd.clone()),
			..Default::default()
		},
		type_: Some("Opaque".to_owned()),
		data: Some(secret_data),
		..Default::default()
	};

	Api::<Secret>::namespaced(kubernetes_client.clone(), namespace)
		.patch(
			&secret_name_for_db_pwd,
			&PatchParams::apply(&secret_name_for_db_pwd),
			&Patch::Apply(secret_spec_for_db_pwd),
		)
		.await?;

	log::trace!("request_id: {request_id} - Creating configmap for database");

	let mut config_data = BTreeMap::new();
	config_data.insert(
		"redis.conf".to_owned(),
		generate_config_data_template("redis.conf"),
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
			name: Some(pvc_prefix_for_db.to_owned()),
			..Default::default()
		},
		spec: Some(PersistentVolumeClaimSpec {
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
			init_containers: Some(vec![Container {
				name: "config".to_owned(),
				image: Some("redis:6.2.3-alpine".to_owned()),
				env: Some(vec![EnvVar {
					name: "USER_PASSWORD".to_owned(),
					value_from: Some(EnvVarSource {
						secret_key_ref: Some(SecretKeySelector {
							name: Some(secret_name_for_db_pwd),
							key: secret_key_for_db_pwd.to_owned(),
							..Default::default()
						}),
						..Default::default()
					}),
					..Default::default()
				}]),
				command: Some(vec!["sh".to_owned(), "-c".to_owned()]),
				args: Some(vec![
					"|".to_owned(),
					generate_command_data_template("init_container"),
				]),
				volume_mounts: Some(vec![
					VolumeMount {
						name: "redis-config".to_owned(),
						mount_path: "/etc/redis/".to_owned(),
						..Default::default()
					},
					VolumeMount {
						name: "config".to_owned(),
						mount_path: "/tmp/redis/".to_owned(),
						..Default::default()
					},
				]),
				..Default::default()
			}]),
			containers: vec![Container {
				name: "redis".to_owned(),
				image: Some("redis:6.2.3-alpine".to_owned()),
				command: Some(vec!["redis-server".to_owned()]),
				args: Some(vec!["/etc/redis/redis.conf".to_owned()]),
				ports: Some(vec![ContainerPort {
					name: Some(sts_port_name_for_db.to_owned()),
					container_port: redis_port,
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
				liveness_probe: Some(Probe {
					exec: Some(ExecAction {
						command: Some(vec![
							"redis-cli".to_owned(),
							"ping".to_owned(),
						]),
					}),
					tcp_socket: Some(TCPSocketAction {
						port: IntOrString::Int(redis_port),
						..Default::default()
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
					tcp_socket: Some(TCPSocketAction {
						port: IntOrString::Int(redis_port),
						..Default::default()
					}),
					initial_delay_seconds: Some(5),
					failure_threshold: Some(10),
					period_seconds: Some(2),
					timeout_seconds: Some(5),
					..Default::default()
				}),
				volume_mounts: Some(vec![
					VolumeMount {
						name: pvc_prefix_for_db.to_owned(),
						mount_path: "/data".to_owned(),
						..Default::default()
					},
					VolumeMount {
						name: "redis-config".to_owned(),
						mount_path: "/etc/redis/".to_owned(),
						..Default::default()
					},
				]),
				..Default::default()
			}],
			volumes: Some(vec![
				Volume {
					name: "config".to_owned(),
					config_map: Some(ConfigMapVolumeSource {
						name: Some(configmap_name_for_db.to_owned()),
						..Default::default()
					}),
					..Default::default()
				},
				Volume {
					name: "redis-config".to_owned(),
					empty_dir: Some(EmptyDirVolumeSource {
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
			replicas: Some(replica_numbers),
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

pub async fn delete_kubernetes_redis_database(
	workspace_id: &Uuid,
	database_id: &Uuid,
	kubeconfig: KubernetesConfigDetails,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client =
		super::super::get_kubernetes_client(kubeconfig.auth_details).await?;

	// names
	let namespace = workspace_id.as_str();
	let secret_name_for_db_pwd = format!("db-pwd-{database_id}");
	let svc_name_for_db = format!("db-{database_id}-service");
	let sts_name_for_db = format!("db-{database_id}-sts");
	let sts_port_name_for_db = format!("db-{database_id}-port");
	let pvc_prefix_for_db = "data";
	let configmap_name_for_db = format!("db-{database_id}-config");

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
		.delete_opt(&secret_name_for_db_pwd, &DeleteParams::default())
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

pub async fn handle_redis_scaling(
	workspace_id: &Uuid,
	database_id: &Uuid,
	kubeconfig: KubernetesConfigDetails,
	request_id: &Uuid,
	replica_numbers: i32,
) -> Result<(), Error> {
	let kubernetes_client =
		super::super::get_kubernetes_client(kubeconfig.auth_details).await?;
	let namespace = workspace_id.as_str();
	let sts_name_for_db = format!("db-{database_id}-sts");
	let labels =
		BTreeMap::from([("database".to_owned(), database_id.to_string())]);

	log::trace!("request_id: {request_id} - Scaling replica for database");

	let statefulset_spec_for_db = StatefulSet {
		metadata: ObjectMeta {
			name: Some(sts_name_for_db.clone()),
			labels: Some(labels.clone()),
			..Default::default()
		},
		spec: Some(StatefulSetSpec {
			replicas: Some(replica_numbers),
			selector: LabelSelector {
				match_labels: Some(labels.clone()),
				..Default::default()
			},
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

fn generate_config_data_template(config: &str) -> String {
	match config {
		"redis.conf" => format!(
			r#" |
			bind 0.0.0.0
            port 6379
            timeout 0
            tcp-keepalive 300
            save 900 1
            save 300 10
            save 60 10000
            "#
		),
		_ => format!(r#"exit"#),
	}
}

fn generate_command_data_template(container: &str) -> String {
	match container {
		"init_container" => format!(
			r#"
            cp /tmp/redis/redis.conf /etc/redis/redis.conf
            echo "masterauth $USER_PASSWORD" >> /etc/redis/redis.conf
            echo "requirepass $USER_PASSWORD" >> /etc/redis/redis.conf
            echo "finding master..."
            MASTER_FDQN=`hostname  -f | sed -e 's/redis-[0-9]\./redis-0./'`
            if [ "$(redis-cli -h sentinel -p 5000 ping)" != "PONG" ]; then
              echo "master not found, defaulting to redis-0"

              if [ "$(hostname)" == "redis-0" ]; then
                echo "this is redis-0, not updating config..."
              else
                echo "updating redis.conf..."
                echo "slaveof $MASTER_FDQN 6379" >> /etc/redis/redis.conf
              fi
            else
              echo "sentinel found, finding master"
              MASTER="$(redis-cli -h sentinel -p 5000 sentinel get-master-addr-by-name mymaster | grep -E '(^redis-\d{{1,}})|([0-9]{{1,3}}\.[0-9]{{1,3}}\.[0-9]{{1,3}}\.[0-9]{{1,3}})')"
              echo "master found : $MASTER, updating redis.conf"
              echo "slaveof $MASTER 6379" >> /etc/redis/redis.conf
            fi
			"#
		),
		_ => format!(r#"exit 1"#),
	}
}
