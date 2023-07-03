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
	api::{AttachParams, Patch, PatchParams},
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

pub async fn patch_kubernetes_psql_database(
	workspace_id: &Uuid,
	database_id: &Uuid,
	db_plan: &DatabasePlanType,
	kubeconfig: Kubeconfig,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client =
		super::super::get_kubernetes_client(kubeconfig).await?;

	// names
	let namespace = workspace_id.as_str();
	let sts_name_for_db = get_database_sts_name(database_id);
	let svc_name_for_db = get_database_service_name(database_id);
	let pvc_claim_for_db = get_database_pvc_name(database_id);

	// constants
	let psql_port = 5432;
	let psql_version = "postgres:14";

	let labels =
		BTreeMap::from([("database".to_owned(), database_id.to_string())]);

	log::trace!("request_id: {request_id} - Creating service for database");

	let service_for_db = Service {
		metadata: ObjectMeta {
			name: Some(svc_name_for_db.to_owned()),
			..Default::default()
		},
		spec: Some(ServiceSpec {
			selector: Some(labels.clone()),
			ports: Some(vec![ServicePort {
				name: Some("postgres".to_owned()),
				port: psql_port,
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
				name: "postgres".to_owned(),
				image: Some(psql_version.to_owned()),
				env: Some(vec![
					EnvVar {
						name: "POSTGRES_USER".to_owned(),
						value: Some("postgres".to_owned()),
						..Default::default()
					},
					EnvVar {
						name: "POSTGRES_PASSWORD".to_owned(),
						value: Some("patr".to_owned()),
						..Default::default()
					},
					EnvVar {
						name: "POSTGRES_HOST_AUTH_METHOD".to_owned(),
						value: Some("scram-sha-256".to_owned()),
						..Default::default()
					},
					EnvVar {
						name: "POSTGRES_INITDB_ARGS".to_owned(),
						value: Some("--auth-host=scram-sha-256".to_owned()),
						..Default::default()
					},
				]),
				ports: Some(vec![ContainerPort {
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
							"bash".to_owned(),
							"-c".to_owned(),
							"psql -w -U postgres -d postgres -c \"SELECT 1\""
								.to_owned(),
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
							"pg_isready -U postgres -d postgres -q".to_owned(),
						]),
					}),
					initial_delay_seconds: Some(5),
					failure_threshold: Some(10),
					period_seconds: Some(2),
					timeout_seconds: Some(5),
					..Default::default()
				}),
				volume_mounts: Some(vec![VolumeMount {
					name: pvc_claim_for_db.to_owned(),
					mount_path: "/var/lib/postgresql/data".to_owned(),
					sub_path: Some("postgres".to_owned()),
					..Default::default()
				}]),
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

pub async fn change_psql_database_password(
	workspace_id: &Uuid,
	database_id: &Uuid,
	kubeconfig: Kubeconfig,
	request_id: &Uuid,
	new_password: &String,
) -> Result<(), Error> {
	log::trace!("request_id: {request_id} - Connecting to Postgres server and changing password");

	let sts_name_for_db = get_database_sts_name(database_id);
	let namespace = workspace_id.as_str();
	let kubernetes_client = get_kubernetes_client(kubeconfig).await?;

	Api::<Pod>::namespaced(kubernetes_client.clone(), namespace)
		.exec(
			&format!("{sts_name_for_db}-0"),
			[
				"bash".to_owned(),
				"-c".to_owned(),
				format!("psql -U postgres -c \"ALTER USER postgres WITH PASSWORD '{new_password}'\"")
			],
			&AttachParams {
				..Default::default()
			},
		)
		.await?;

	log::trace!("request_id: {request_id} - Password changed successfully");
	Ok(())
}
