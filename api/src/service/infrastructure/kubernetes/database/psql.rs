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
	api::{DeleteParams, ListParams, Patch, PatchParams},
	config::Kubeconfig,
	core::ObjectMeta,
	Api,
};

use crate::{
	service::{ext_traits::DeleteOpt, ResourceLimitsForPlan},
	utils::Error,
};

pub async fn create_kubernetes_psql_database(
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
	let secret_name_for_db_pwd = format!("db-pwd-{database_id}");
	let svc_name_for_db = format!("db-{database_id}");
	let sts_name_for_db = format!("db-{database_id}");
	let sts_port_name_for_db = "postgresql".to_string();
	let pvc_prefix_for_db = "pvc"; // actual name will be `pvc-{sts_name_for_db}-{sts_ordinal}`
	let configmap_name_for_db = "master-slave-config".to_string();

	// constants
	let secret_key_for_db_pwd = "password";
	let psql_port = 5432;

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

	let mut config_data = BTreeMap::new();
	config_data.insert(
		"master-slave-config.sh".to_owned(),
		vec![
			"HOST=`hostname -s`".to_owned(),
            "ORD=${HOST##*-}".to_owned(),
            "HOST_TEMPLATE=${HOST%-*}".to_owned(),
            "case $ORD in".to_owned(),
                "0)".to_owned(),
                r#"echo "host    replication     all     all     md5" >> /var/lib/postgresql/data/pg_hba.conf"#.to_owned(),
                r#"echo "archive_mode = on"  >> /etc/postgresql/postgresql.conf"#.to_owned(),
                r#"echo "archive_mode = on"  >> /etc/postgresql/postgresql.conf"#.to_owned(),
                r#"echo "archive_command = '/bin/true'"  >> /etc/postgresql/postgresql.conf"#.to_owned(),
                r#"echo "archive_timeout = 0"  >> /etc/postgresql/postgresql.conf"#.to_owned(),
                r#"echo "max_wal_senders = 8"  >> /etc/postgresql/postgresql.conf"#.to_owned(),
                r#"echo "wal_keep_segments = 32"  >> /etc/postgresql/postgresql.conf"#.to_owned(),
                r#"echo "wal_level = hot_standby"  >> /etc/postgresql/postgresql.conf"#.to_owned(),
                ";;".to_owned(),
                "*)".to_owned(),
                "# stop initial server to copy data".to_owned(),
                "pg_ctl -D /var/lib/postgresql/data/ -m fast -w stop".to_owned(),
                "rm -rf /var/lib/postgresql/data/*".to_owned(),
                "# add service name for DNS resolution".to_owned(),
                format!("PGPASSWORD=k8s-postgres-ha pg_basebackup -h ${{HOST_TEMPLATE}}-0.{svc_name_for_db} -w -U replicator -p 5432 -D /var/lib/postgresql/data -Fp -Xs -P -R"),
                "# start server to keep container's screep happy".to_owned(),
                "pg_ctl -D /var/lib/postgresql/data/ -w start".to_owned(),
                ";;".to_owned(),
            "esac".to_owned(),
		].join("\n"),
	);
	config_data.insert(
		"create-replication-role.sql".to_owned(),
		vec![
			"CREATE USER replicator WITH REPLICATION ENCRYPTED PASSWORD 'k8s-postgres-ha';".to_owned()
		].join("\n")
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
			selector: Some(labels.clone()),
			ports: Some(vec![ServicePort {
				name: Some("postgresql".to_owned()),
				port: psql_port,
				target_port: Some(IntOrString::Int(psql_port)),
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
			containers: vec![Container {
				name: "pg-db".to_owned(),
				image: Some("postgres:12".to_owned()),
				env: Some(vec![EnvVar {
					name: "POSTGRES_PASSWORD".to_owned(),
					value: Some("test".to_owned()),
					value_from: Some(EnvVarSource {
						secret_key_ref: Some(SecretKeySelector {
							name: Some(secret_name_for_db_pwd),
							key: secret_key_for_db_pwd.to_owned(),
							..Default::default()
						}),
						..Default::default()
					}),
				}]),
				ports: Some(vec![ContainerPort {
					name: Some(sts_port_name_for_db.to_owned()),
					container_port: psql_port,
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
							"psql".to_owned(),
							"-w".to_owned(),
							"-U".to_owned(),
							"postgres".to_owned(),
							"-d".to_owned(),
							"postgres".to_owned(),
							"-c".to_owned(),
							"SELECT 1".to_owned(),
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
							"pg_isready".to_owned(),
							"-U".to_owned(),
							"postgres".to_owned(),
							"-d".to_owned(),
							"postgres".to_owned(),
							"-q".to_owned(),
						]),
					}),
					initial_delay_seconds: Some(5),
					failure_threshold: Some(10),
					period_seconds: Some(2),
					timeout_seconds: Some(5),
					..Default::default()
				}),
				volume_mounts: Some(vec![VolumeMount {
					name: "init-scripts".to_owned(),
					mount_path: "/docker-entrypoint-initdb.d".to_owned(),
					..Default::default()
				}]),
				..Default::default()
			}],
			volumes: Some(vec![Volume {
				name: "init-scripts".to_owned(),
				config_map: Some(ConfigMapVolumeSource {
					name: Some(configmap_name_for_db.to_owned()),
					..Default::default()
				}),
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

pub async fn delete_kubernetes_psql_database(
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

pub async fn handle_psql_scaling(
	workspace_id: &Uuid,
	database_id: &Uuid,
	kubeconfig: Kubeconfig,
	request_id: &Uuid,
	replica_numbers: i32,
) -> Result<(), Error> {
	let kubernetes_client =
		super::super::get_kubernetes_client(kubeconfig).await?;
	let namespace = workspace_id.as_str();
	let sts_name_for_db = format!("db-{database_id}");
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
