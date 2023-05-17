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
			ExecAction,
			PersistentVolumeClaim,
			PersistentVolumeClaimSpec,
			PodSpec,
			PodTemplateSpec,
			Probe,
			ResourceRequirements,
			Secret,
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
	replica_numbers: i32,
) -> Result<(), Error> {
	let kubernetes_client =
		super::super::get_kubernetes_client(kubeconfig).await?;

	// names
	let namespace = workspace_id.as_str();
	let secret_name_for_db_pwd = "mysql".to_owned();
	let master_svc_name_for_db = "mysql".to_owned();
	let slave_svc_name_for_db = "mysql-read".to_owned();
	let sts_name_for_db = "mysql".to_owned();
	let pvc_prefix_for_db = "pvc"; // actual name will be `pvc-{sts_name_for_db}-{sts_ordinal}`
	let configmap_name_for_db = "mysql".to_owned();

	// constants
	let secret_key_for_db_pwd = "password";
	let mysql_port = 3306;

	// plan
	let (db_ram, db_cpu, db_volume) = db_plan.get_resource_limits();
	let labels =
		BTreeMap::from([("database".to_owned(), database_id.to_string())]);

	log::trace!("request_id: {request_id} - Creating secret for database pwd");

	let secret_spec_for_db_pwd = Secret {
		metadata: ObjectMeta {
			name: Some(secret_name_for_db_pwd.clone()),
			..Default::default()
		},
		string_data: Some(
			[(secret_key_for_db_pwd.to_owned(), db_pwd.into())].into(),
		),
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

	let mut config_label = BTreeMap::new();
	config_label.insert("database".to_owned(), database_id.to_string());
	config_label
		.insert("app.kubernetes.io/name".to_owned(), "mysql".to_owned());

	let mut config_data = BTreeMap::new();
	config_data.insert(
		"primary.cnf".to_owned(),
		vec!["[mysqld]", "log-bin"].join("\n"),
	);
	config_data.insert(
		"replica.cnf".to_owned(),
		vec!["[mysqld]", "super-read-only"].join("\n"),
	);

	let config_for_db = ConfigMap {
		metadata: ObjectMeta {
			name: Some(configmap_name_for_db.to_owned()),
			labels: Some(config_label.clone()),
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

	log::trace!("request_id: {request_id} - Creating read and write service for master database");

	let master_service_for_db = Service {
		metadata: ObjectMeta {
			name: Some(master_svc_name_for_db.to_owned()),
			labels: Some(config_label.clone()),
			..Default::default()
		},
		spec: Some(ServiceSpec {
			cluster_ip: Some("None".to_owned()),
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
			&master_svc_name_for_db,
			&PatchParams::apply(&master_svc_name_for_db),
			&Patch::Apply(master_service_for_db),
		)
		.await?;

	log::trace!(
		"request_id: {request_id} - Creating read service for slave database"
	);

	let mut slave_config_label = config_label.clone();
	slave_config_label.insert("readonly".to_owned(), "true".to_owned());

	let slave_service_for_db = Service {
		metadata: ObjectMeta {
			name: Some(slave_svc_name_for_db.to_owned()),
			labels: Some(slave_config_label.clone()),
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
			&slave_svc_name_for_db,
			&PatchParams::apply(&slave_svc_name_for_db),
			&Patch::Apply(slave_service_for_db),
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
			labels: Some(config_label.clone()),
			..Default::default()
		}),
		spec: Some(PodSpec {
			init_containers: Some(vec![
				Container {
					name: "init-mysql".to_owned(),
					image: Some("mysql:5.7".to_owned()),
					command: Some(vec![
						"bash".to_owned(),
						"-c".to_owned(),
						vec![
							"set -ex".to_owned(),
							"[[ $HOSTNAME =~ -([0-9]+)$ ]] || exit 1".to_owned(),
							"ordinal=${BASH_REMATCH[1]}".to_owned(),
							"echo [mysqld] > /mnt/conf.d/server-id.cnf".to_owned(),
							r#"echo "" >> /mnt/conf.d/server-id.cnf"#.to_owned(),
							"echo server-id=$((100 + $ordinal)) >> /mnt/conf.d/server-id.cnf".to_owned(),
							"if [[ $ordinal -eq 0 ]]; then".to_owned(),
							"cp /mnt/config-map/primary.cnf /mnt/conf.d/".to_owned(),
							"else".to_owned(),
							"cp /mnt/config-map/replica.cnf /mnt/conf.d/".to_owned(),
							"fi".to_owned()
						].join("\n")
					]),
					volume_mounts: Some(vec![
						VolumeMount {
							name: "conf".to_owned(),
							mount_path: "/mnt/conf.d".to_owned(),
							..Default::default()
						},
						VolumeMount {
							name: "config-map".to_owned(),
							mount_path: "/mnt/config-map".to_owned(),
							..Default::default()
						},
					]),
					..Default::default()
				},
				Container {
					name: "clone-mysql".to_owned(),
					image: Some(
						"gcr.io/google-samples/xtrabackup:1.0".to_owned(),
					),
					command: Some(vec![
						"bash".to_owned(),
						"-c".to_owned(),
						vec![
							"set -ex".to_owned(),
							"[[ -d /var/lib/mysql/mysql ]] && exit 0".to_owned(),
							"[[ `hostname` =~ -([0-9]+)$ ]] || exit 1".to_owned(),
							"ordinal=${BASH_REMATCH[1]}".to_owned(),
							"[[ $ordinal -eq 0 ]] && exit 0".to_owned(),
							format!("ncat --recv-only {sts_name_for_db}-$(($ordinal-1)).{sts_name_for_db}.{namespace}.svc.cluster.local 3307 | xbstream -x -C /var/lib/mysql"),
							"xtrabackup --prepare --target-dir=/var/lib/mysql".to_owned()
						].join("\n")
					]),
					volume_mounts: Some(vec![
						VolumeMount {
							name: pvc_prefix_for_db.to_owned(),
							mount_path: "/var/lib/mysql".to_owned(),
							sub_path: Some("mysql".to_owned()),
							..Default::default()
						},
						VolumeMount {
							name: "conf".to_owned(),
							mount_path: "/etc/mysql/conf.d".to_owned(),
							..Default::default()
						},
					]),
					..Default::default()
				},
			]),
			containers: vec![
				Container {
					name: "mysql".to_owned(),
					image: Some("mysql:5.7".to_owned()),
					env: Some(vec![EnvVar {
						name: "MYSQL_ALLOW_EMPTY_PASSWORD".to_owned(),
						value: Some("1".to_owned()),
						..Default::default()
					}]),
					ports: Some(vec![ContainerPort {
						name: Some("mysql".to_owned()),
						container_port: mysql_port,
						..Default::default()
					}]),
					volume_mounts: Some(vec![
						VolumeMount {
							name: pvc_prefix_for_db.to_owned(),
							mount_path: "/var/lib/mysql".to_owned(),
							sub_path: Some("mysql".to_owned()),
							..Default::default()
						},
						VolumeMount {
							name: "conf".to_owned(),
							mount_path: "/etc/mysql/conf.d".to_owned(),
							..Default::default()
						},
					]),
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
								(
									"memory".to_string(),
									Quantity("25M".to_owned()),
								),
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
								"mysqladmin".to_owned(),
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
								"mysql".to_owned(),
								"-h".to_owned(),
								"127.0.0.1".to_owned(),
								"-e".to_owned(),
								"SELECT 1".to_owned(),
							]),
						}),
						initial_delay_seconds: Some(5),
						failure_threshold: Some(10),
						period_seconds: Some(2),
						timeout_seconds: Some(1),
						..Default::default()
					}),
					..Default::default()
				},
				Container {
					name: "xtrabackup".to_owned(),
					image: Some(
						"gcr.io/google-samples/xtrabackup:1.0".to_owned(),
					),
					ports: Some(vec![ContainerPort {
						name: Some("xtrabackup".to_owned()),
						container_port: mysql_port + 1,
						..Default::default()
					}]),
					command: Some(vec![
						"bash".to_owned(),
						"-c".to_owned(),
						vec![
							"set -ex".to_owned(),
							"cd /var/lib/mysql".to_owned(),
							r#"if [[ -f xtrabackup_slave_info && "x$(<xtrabackup_slave_info)" != "x" ]]; then"#.to_owned(),
							"cat xtrabackup_slave_info | sed -E 's/;$//g' > change_master_to.sql.in".to_owned(),
							"rm -f xtrabackup_slave_info xtrabackup_binlog_info".to_owned(),
							"elif [[ -f xtrabackup_binlog_info ]]; then".to_owned(),
							"[[ `cat xtrabackup_binlog_info` =~ ^(.*?)[[:space:]]+(.*?)$ ]] || exit 1".to_owned(),
							"rm -f xtrabackup_binlog_info xtrabackup_slave_info".to_owned(),
							r#"echo "CHANGE MASTER TO MASTER_LOG_FILE='${BASH_REMATCH[1]}',\"#.to_owned(),
							r#"MASTER_LOG_POS=${BASH_REMATCH[2]}" > change_master_to.sql.in"#.to_owned(),
							"fi".to_owned(),
							"if [[ -f change_master_to.sql.in ]]; then".to_owned(),
							r#"until mysql -h 127.0.0.1 -e "SELECT 1"; do sleep 1; done"#.to_owned(),
							r#"mysql -h 127.0.0.1 \"#.to_owned(),
							r#"-e "$(<change_master_to.sql.in), \"#.to_owned(),
							format!(r#"MASTER_HOST='{sts_name_for_db}-0.{sts_name_for_db}.{namespace}', \"#),
							r#"MASTER_USER='root', \"#.to_owned(),
							r#"MASTER_PASSWORD='', \"#.to_owned(),
							r#"MASTER_CONNECT_RETRY=10; \"#.to_owned(),
							r#"START SLAVE;" || exit 1"#.to_owned(),
							"mv change_master_to.sql.in change_master_to.sql.orig".to_owned(),
							"fi".to_owned(),
							r#"exec ncat --listen --keep-open --send-only --max-conns=1 3307 -c \"#.to_owned(),
							r#""xtrabackup --backup --slave-info --stream=xbstream --host=127.0.0.1 --user=root""#.to_owned()
						].join("\n")
					]),
					// Failing to make connection because name being cut
					volume_mounts: Some(vec![
						VolumeMount {
							name: pvc_prefix_for_db.to_owned(),
							mount_path: "/var/lib/mysql".to_owned(),
							sub_path: Some("mysql".to_owned()),
							..Default::default()
						},
						VolumeMount {
							name: "conf".to_owned(),
							mount_path: "/etc/mysql/conf.d".to_owned(),
							..Default::default()
						},
					]),
					resources: Some(ResourceRequirements {
						requests: Some(
							[
								(
									"memory".to_string(),
									Quantity("25M".to_owned()),
								),
								("cpu".to_string(), Quantity("50m".to_owned())),
							]
							.into(),
						),
						..Default::default()
					}),
					..Default::default()
				},
			],
			volumes: Some(vec![
				Volume {
					name: "conf".to_owned(),
					empty_dir: Some(EmptyDirVolumeSource {
						..Default::default()
					}),
					..Default::default()
				},
				Volume {
					name: "config-map".to_owned(),
					config_map: Some(ConfigMapVolumeSource {
						name: Some(configmap_name_for_db.to_owned()),
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
			service_name: master_svc_name_for_db,
			replicas: Some(replica_numbers),
			selector: LabelSelector {
				match_labels: Some(config_label.clone()),
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
		super::super::get_kubernetes_client(kubeconfig).await?;

	// names
	let namespace = workspace_id.as_str();
	let secret_name_for_db_pwd = format!("db-pwd-{database_id}");
	let master_svc_name_for_db = format!("db-{database_id}");
	let slave_svc_name_for_db = format!("db-{database_id}-read");
	let sts_name_for_db = format!("db-{database_id}");

	let label = format!("database={}", database_id);

	log::trace!("request_id: {request_id} - Deleting statefulset for database");
	Api::<StatefulSet>::namespaced(kubernetes_client.clone(), namespace)
		.delete_opt(&sts_name_for_db, &DeleteParams::default())
		.await?;

	log::trace!(
		"request_id: {request_id} - Deleting master service for database"
	);
	Api::<Service>::namespaced(kubernetes_client.clone(), namespace)
		.delete_opt(&master_svc_name_for_db, &DeleteParams::default())
		.await?;

	log::trace!(
		"request_id: {request_id} - Deleting slave service for database"
	);
	Api::<Service>::namespaced(kubernetes_client.clone(), namespace)
		.delete_opt(&slave_svc_name_for_db, &DeleteParams::default())
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

pub async fn handle_scaling(
	workspace_id: &Uuid,
	database_id: &Uuid,
	kubeconfig: Kubeconfig,
	request_id: &Uuid,
	replica_numbers: i32,
) -> Result<(), Error> {
	log::trace!("request_id: {request_id} - Handling replica changes");
	let kubernetes_client =
		super::super::get_kubernetes_client(kubeconfig).await?;
	let namespace = workspace_id.as_str();
	let sts_name_for_db = format!("db-{database_id}");

	let mut config_label = BTreeMap::new();
	config_label.insert("database".to_owned(), database_id.to_string());
	config_label
		.insert("app.kubernetes.io/name".to_owned(), "mysql".to_owned());

	let statefulset_spec_for_db = StatefulSet {
		metadata: ObjectMeta {
			name: Some(sts_name_for_db.clone()),
			..Default::default()
		},
		spec: Some(StatefulSetSpec {
			replicas: Some(replica_numbers),
			selector: LabelSelector {
				match_labels: Some(config_label.clone()),
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
