use std::collections::BTreeMap;

use api_models::{
	models::workspace::infrastructure::deployment::{
		Deployment,
		DeploymentRunningDetails,
		DeploymentStatus,
		EnvironmentVariableValue,
		ExposedPortType,
	},
	utils::Uuid,
};
use eve_rs::AsError;
use k8s_openapi::{
	api::{
		apps::v1::{
			Deployment as K8sDeployment,
			DeploymentSpec,
			DeploymentStrategy,
			RollingUpdateDeployment,
			StatefulSet,
			StatefulSetSpec,
			StatefulSetUpdateStrategy,
		},
		autoscaling::v1::{
			CrossVersionObjectReference,
			HorizontalPodAutoscaler,
			HorizontalPodAutoscalerSpec,
		},
		core::v1::{
			ConfigMap,
			ConfigMapVolumeSource,
			Container,
			ContainerPort,
			EnvVar,
			HTTPGetAction,
			KeyToPath,
			LocalObjectReference,
			PersistentVolumeClaim,
			PersistentVolumeClaimSpec,
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
		networking::v1::{
			HTTPIngressPath,
			HTTPIngressRuleValue,
			Ingress,
			IngressBackend,
			IngressRule,
			IngressServiceBackend,
			IngressSpec,
			ServiceBackendPort,
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
	api::{DeleteParams, ListParams, Patch, PatchParams, PropagationPolicy},
	config::Kubeconfig,
	core::{ErrorResponse, ObjectMeta},
	Api,
	Error as KubeError,
};
use sha2::{Digest, Sha512};

use crate::{
	db::{self, DeploymentVolume},
	error,
	models::deployment,
	service::{self, ext_traits::DeleteOpt},
	utils::{constants::request_keys, settings::Settings, Error},
	Database,
};

pub async fn update_kubernetes_deployment(
	workspace_id: &Uuid,
	deployment: &Deployment,
	full_image: &str,
	digest: Option<&str>,
	running_details: &DeploymentRunningDetails,
	deployment_volumes: &Vec<DeploymentVolume>,
	kubeconfig: Kubeconfig,
	deployed_region_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client = super::get_kubernetes_client(kubeconfig).await?;

	// the namespace is workspace id
	let namespace = workspace_id.as_str();

	log::trace!(
		"Deploying the container with id: {} with request_id: {}",
		deployment.id,
		request_id,
	);

	let image_name = if let Some(digest) = digest {
		format!("{}@{}", full_image, digest)
	} else {
		full_image.to_string()
	};

	let kubernetes_config_map = ConfigMap {
		metadata: ObjectMeta {
			name: Some(format!("config-mount-{}", deployment.id)),
			namespace: Some(namespace.to_string()),
			..ObjectMeta::default()
		},
		binary_data: Some(
			running_details
				.config_mounts
				.iter()
				.map(|(path, data)| {
					(path.to_string(), ByteString(data.clone().into()))
				})
				.collect(),
		),
		..ConfigMap::default()
	};

	log::trace!(
		"request_id: {} - Patching ConfigMap for deployment with id: {}",
		request_id,
		deployment.id
	);

	let mut hasher = Sha512::new();
	if let Some(configs) = &kubernetes_config_map.binary_data {
		configs.iter().for_each(|(key, value)| {
			hasher.update(key.as_bytes());
			hasher.update(&value.0);
		})
	}

	let config_map_hash = hex::encode(hasher.finalize());

	Api::<ConfigMap>::namespaced(kubernetes_client.clone(), namespace)
		.patch(
			&format!("config-mount-{}", deployment.id),
			&PatchParams::apply(&format!("config-mount-{}", deployment.id)),
			&Patch::Apply(kubernetes_config_map),
		)
		.await?;

	// get this from machine type
	let (cpu_count, memory_count) = deployment::MACHINE_TYPES
		.get()
		.unwrap()
		.get(&deployment.machine_type)
		.unwrap_or(&(1, 2));
	let machine_type = [
		(
			"memory".to_string(),
			Quantity(format!("{:.1}G", (*memory_count as f64) / 4f64)),
		),
		(
			"cpu".to_string(),
			Quantity(format!("{:.1}", *cpu_count as f64)),
		),
	]
	.into_iter()
	.collect::<BTreeMap<_, _>>();

	log::trace!(
		"request_id: {} - Deploying deployment: {}",
		request_id,
		deployment.id,
	);

	let labels = [
		(
			request_keys::DEPLOYMENT_ID.to_string(),
			deployment.id.to_string(),
		),
		(
			request_keys::WORKSPACE_ID.to_string(),
			workspace_id.to_string(),
		),
		(
			request_keys::REGION.to_string(),
			deployment.region.to_string(),
		),
		("app.kubernetes.io/name".to_string(), "vault".to_string()),
	]
	.into_iter()
	.collect::<BTreeMap<_, _>>();

	log::trace!(
		"request_id: {} - generating deployment configuration",
		request_id
	);

	let annotations = [
		(
			"vault.security.banzaicloud.io/vault-addr".to_string(),
			config.vault.host.clone(),
		),
		(
			"vault.security.banzaicloud.io/vault-role".to_string(),
			"vault".to_string(),
		),
		(
			"vault.security.banzaicloud.io/vault-skip-verify".to_string(),
			"false".to_string(),
		),
		(
			"vault.security.banzaicloud.io/vault-agent".to_string(),
			"false".to_string(),
		),
		(
			"vault.security.banzaicloud.io/vault-path".to_string(),
			"kubernetes".to_string(),
		),
	]
	.into_iter()
	.collect();

	if !deployment_volumes.is_empty() {
		// Deleting STS using orphan flag to update the volumes
		log::trace!(
			"request_id: {} deleting sts without deleting the pod",
			request_id
		);
		Api::<StatefulSet>::namespaced(
			kubernetes_client.clone(),
			workspace_id.as_str(),
		)
		.delete_opt(
			&format!("sts-{}", deployment.id),
			&DeleteParams {
				propagation_policy: Some(PropagationPolicy::Orphan),
				..DeleteParams::default()
			},
		)
		.await?;
	}

	let mut volume_mounts: Vec<VolumeMount> = Vec::new();
	let mut volumes: Vec<Volume> = Vec::new();
	let mut pvc: Vec<PersistentVolumeClaim> = Vec::new();

	if !running_details.config_mounts.is_empty() {
		volume_mounts.push(VolumeMount {
			name: "config-mounts".to_string(),
			mount_path: "/etc/config".to_string(),
			..VolumeMount::default()
		});
		volumes.push(Volume {
			name: "config-mounts".to_string(),
			config_map: Some(ConfigMapVolumeSource {
				name: Some(format!("config-mount-{}", deployment.id)),
				items: Some(
					running_details
						.config_mounts
						.keys()
						.map(|path| KeyToPath {
							key: path.clone(),
							path: path.clone(),
							..KeyToPath::default()
						})
						.collect(),
				),
				..ConfigMapVolumeSource::default()
			}),
			..Volume::default()
		})
	}

	for volume in deployment_volumes {
		volume_mounts.push(VolumeMount {
			name: format!("pvc-{}", volume.volume_id),
			// make sure user does not have the mount_path in the directory
			// in the fs, by my observation it gives crashLoopBackOff error
			mount_path: volume.path.to_string(),
			..VolumeMount::default()
		});

		let storage_limit = [(
			"storage".to_string(),
			Quantity(format!("{}Gi", volume.size)),
		)]
		.into_iter()
		.collect::<BTreeMap<_, _>>();

		pvc.push(PersistentVolumeClaim {
			metadata: ObjectMeta {
				name: Some(format!("pvc-{}", volume.volume_id)),
				namespace: Some(workspace_id.to_string()),
				..ObjectMeta::default()
			},
			spec: Some(PersistentVolumeClaimSpec {
				access_modes: Some(vec!["ReadWriteOnce".to_string()]),
				resources: Some(ResourceRequirements {
					requests: Some(storage_limit),
					..ResourceRequirements::default()
				}),
				..PersistentVolumeClaimSpec::default()
			}),
			..PersistentVolumeClaim::default()
		});
	}

	// Creating service before deployment and sts as sts need service to be
	// present before sts is created
	let kubernetes_service = Service {
		metadata: ObjectMeta {
			name: Some(format!("service-{}", deployment.id)),
			..ObjectMeta::default()
		},
		spec: Some(ServiceSpec {
			ports: Some(
				running_details
					.ports
					.keys()
					.map(|port| ServicePort {
						port: port.value() as i32,
						target_port: Some(
							IntOrString::Int(port.value() as i32),
						),
						name: Some(format!("port-{}", port)),
						..ServicePort::default()
					})
					.collect::<Vec<_>>(),
			),
			selector: Some(labels.clone()),
			cluster_ip: if running_details.volumes.is_empty() {
				None
			} else {
				Some("None".to_string())
			},
			..ServiceSpec::default()
		}),
		..Service::default()
	};

	// Create the service defined above
	log::trace!("request_id: {} - creating ClusterIp service", request_id);
	let service_api =
		Api::<Service>::namespaced(kubernetes_client.clone(), namespace);

	service_api
		.patch(
			&format!("service-{}", deployment.id),
			&PatchParams::apply(&format!("service-{}", deployment.id)),
			&Patch::Apply(kubernetes_service),
		)
		.await?;

	let metadata = ObjectMeta {
		name: Some(format!(
			"{}-{}",
			if running_details.volumes.is_empty() {
				"deployment"
			} else {
				"sts"
			},
			deployment.id
		)),
		namespace: Some(namespace.to_string()),
		labels: Some(labels.clone()),
		..ObjectMeta::default()
	};
	let replicas = Some(running_details.min_horizontal_scale as i32);
	let selector = LabelSelector {
		match_expressions: None,
		match_labels: Some(labels.clone()),
	};
	let template = PodTemplateSpec {
		spec: Some(PodSpec {
			containers: vec![Container {
				name: format!(
					"{}-{}",
					if running_details.volumes.is_empty() {
						"deployment"
					} else {
						"sts"
					},
					deployment.id
				),
				image: Some(image_name),
				image_pull_policy: Some("Always".to_string()),
				ports: Some(
					running_details
						.ports
						.keys()
						.map(|port| ContainerPort {
							container_port: port.value() as i32,
							..ContainerPort::default()
						})
						.collect::<Vec<_>>(),
				),
				startup_probe: running_details.startup_probe.as_ref().map(
					|probe| Probe {
						http_get: Some(HTTPGetAction {
							path: Some(probe.path.clone()),
							port: IntOrString::Int(probe.port as i32),
							scheme: Some("HTTP".to_string()),
							..HTTPGetAction::default()
						}),
						failure_threshold: Some(15),
						period_seconds: Some(10),
						timeout_seconds: Some(3),
						..Probe::default()
					},
				),
				liveness_probe: running_details.liveness_probe.as_ref().map(
					|probe| Probe {
						http_get: Some(HTTPGetAction {
							path: Some(probe.path.clone()),
							port: IntOrString::Int(probe.port as i32),
							scheme: Some("HTTP".to_string()),
							..HTTPGetAction::default()
						}),
						failure_threshold: Some(15),
						period_seconds: Some(10),
						timeout_seconds: Some(3),
						..Probe::default()
					},
				),
				env: Some(
					running_details
						.environment_variables
						.iter()
						.map(|(name, value)| {
							use EnvironmentVariableValue::*;
							EnvVar {
								name: name.clone(),
								value: Some(match value {
									String(value) => value.clone(),
									Secret { from_secret } => {
										format!(
											"vault:secret/data/{}/{}#data",
											workspace_id, from_secret
										)
									}
								}),
								..EnvVar::default()
							}
						})
						.chain([
							EnvVar {
								name: "PATR".to_string(),
								value: Some("true".to_string()),
								..EnvVar::default()
							},
							EnvVar {
								name: "WORKSPACE_ID".to_string(),
								value: Some(workspace_id.to_string()),
								..EnvVar::default()
							},
							EnvVar {
								name: "DEPLOYMENT_ID".to_string(),
								value: Some(deployment.id.to_string()),
								..EnvVar::default()
							},
							EnvVar {
								name: "DEPLOYMENT_NAME".to_string(),
								value: Some(deployment.name.clone()),
								..EnvVar::default()
							},
							EnvVar {
								name: "CONFIG_MAP_HASH".to_string(),
								value: Some(config_map_hash),
								..EnvVar::default()
							},
						])
						.collect::<Vec<_>>(),
				),
				resources: Some(ResourceRequirements {
					limits: Some(machine_type),
					// https://blog.kubecost.com/blog/requests-and-limits/#the-tradeoffs
					// using too low values for resource request
					// will result in frequent pod restarts if
					// memory usage increases and may result in
					// starvation
					//
					// currently used 5% of the mininum deployment
					// machine type as a request values
					requests: Some(
						[
							("memory".to_string(), Quantity("25M".to_owned())),
							("cpu".to_string(), Quantity("50m".to_owned())),
						]
						.into_iter()
						.collect(),
					),
				}),
				volume_mounts: if !volume_mounts.is_empty() {
					Some(volume_mounts)
				} else {
					None
				},
				..Container::default()
			}],
			volumes: if !volumes.is_empty() {
				Some(volumes)
			} else {
				None
			},
			image_pull_secrets: deployment.registry.is_patr_registry().then(
				|| {
					// TODO: for now patr registry is not supported
					// for user clusters, need to create a separate
					// secret for each private repo in future
					vec![LocalObjectReference {
						name: Some("patr-regcred".to_string()),
					}]
				},
			),
			..PodSpec::default()
		}),
		metadata: Some(ObjectMeta {
			labels: Some(labels.clone()),
			annotations: Some(annotations),
			..ObjectMeta::default()
		}),
	};

	if running_details.volumes.is_empty() {
		let kubernetes_deployment = K8sDeployment {
			metadata,
			spec: Some(DeploymentSpec {
				replicas,
				selector,
				template,
				strategy: Some(DeploymentStrategy {
					type_: Some("RollingUpdate".to_owned()),
					rolling_update: Some(RollingUpdateDeployment {
						max_surge: Some(IntOrString::Int(1)),
						max_unavailable: Some(IntOrString::String(
							"25%".to_owned(),
						)),
					}),
				}),
				..DeploymentSpec::default()
			}),
			..K8sDeployment::default()
		};

		// Create the deployment defined above
		log::trace!("request_id: {} - creating deployment", request_id);
		let deployment_api = Api::<K8sDeployment>::namespaced(
			kubernetes_client.clone(),
			namespace,
		);

		deployment_api
			.patch(
				&format!("deployment-{}", deployment.id),
				&PatchParams::apply(&format!("deployment-{}", deployment.id)),
				&Patch::Apply(kubernetes_deployment),
			)
			.await?;

		// This is because if a user wanted to delete the volume from there sts
		// then a sts will be converted to deployment
		log::trace!(
			"request_id: {} - deleting the stateful set if there are any",
			request_id
		);

		Api::<StatefulSet>::namespaced(
			kubernetes_client.clone(),
			workspace_id.as_str(),
		)
		.delete_opt(&format!("sts-{}", deployment.id), &DeleteParams::default())
		.await?;

		// HPA - horizontal pod autoscaler
		let kubernetes_hpa = HorizontalPodAutoscaler {
			metadata: ObjectMeta {
				name: Some(format!("hpa-{}", deployment.id)),
				namespace: Some(namespace.to_string()),
				..ObjectMeta::default()
			},
			spec: Some(HorizontalPodAutoscalerSpec {
				scale_target_ref: CrossVersionObjectReference {
					api_version: Some("apps/v1".to_string()),
					kind: "Deployment".to_string(),
					name: format!("deployment-{}", deployment.id),
				},
				min_replicas: Some(running_details.min_horizontal_scale.into()),
				max_replicas: running_details.max_horizontal_scale.into(),
				target_cpu_utilization_percentage: Some(80),
			}),
			..HorizontalPodAutoscaler::default()
		};

		// Create the HPA defined above
		log::trace!(
			"request_id: {} - creating horizontal pod autoscalar",
			request_id
		);
		let hpa_api = Api::<HorizontalPodAutoscaler>::namespaced(
			kubernetes_client.clone(),
			namespace,
		);

		hpa_api
			.patch(
				&format!("hpa-{}", deployment.id),
				&PatchParams::apply(&format!("hpa-{}", deployment.id)),
				&Patch::Apply(kubernetes_hpa),
			)
			.await?;
	} else {
		let kubernetes_sts = StatefulSet {
			metadata,
			spec: Some(StatefulSetSpec {
				replicas,
				selector,
				service_name: format!("service-{}", deployment.id),
				template,
				update_strategy: Some(StatefulSetUpdateStrategy {
					type_: Some("RollingUpdate".to_owned()),
					..StatefulSetUpdateStrategy::default()
				}),
				volume_claim_templates: Some(pvc),
				..StatefulSetSpec::default()
			}),
			..StatefulSet::default()
		};

		// Create the statefulset defined above
		log::trace!("request_id: {} - creating statefulset", request_id);
		let sts_api = Api::<StatefulSet>::namespaced(
			kubernetes_client.clone(),
			namespace,
		);

		sts_api
			.patch(
				&format!("sts-{}", deployment.id),
				&PatchParams::apply(&format!("sts-{}", deployment.id)),
				&Patch::Apply(kubernetes_sts),
			)
			.await?;

		// This is because if a user wanted to add the volume to there
		// deployment then a deployment will be converted to sts
		log::trace!(
			"request_id: {} - deleting the deployment set if there are any",
			request_id
		);

		Api::<K8sDeployment>::namespaced(
			kubernetes_client.clone(),
			workspace_id.as_str(),
		)
		.delete_opt(
			&format!("deployment-{}", deployment.id),
			&DeleteParams::default(),
		)
		.await?;

		// Delete the HPA, if any
		log::trace!(
			"request_id: {} - deleting horizontal pod autoscalar",
			request_id
		);
		let hpa_api = Api::<HorizontalPodAutoscaler>::namespaced(
			kubernetes_client.clone(),
			namespace,
		);

		hpa_api
			.delete_opt(
				&format!("hpa-{}", deployment.id),
				&DeleteParams::default(),
			)
			.await?;
	}

	let annotations = [(
		"kubernetes.io/ingress.class".to_string(),
		"nginx".to_string(),
	)]
	.into();

	let ingress_rules = running_details
		.ports
		.iter()
		.filter(|(_, port_type)| *port_type == &ExposedPortType::Http)
		.map(|(port, _)| IngressRule {
			host: Some(format!(
				"{}-{}.{}.{}",
				port,
				deployment.id,
				deployed_region_id,
				config.cloudflare.onpatr_domain
			)),
			http: Some(HTTPIngressRuleValue {
				paths: vec![HTTPIngressPath {
					backend: IngressBackend {
						service: Some(IngressServiceBackend {
							name: format!("service-{}", deployment.id),
							port: Some(ServiceBackendPort {
								number: Some(port.value() as i32),
								..ServiceBackendPort::default()
							}),
						}),
						..Default::default()
					},
					path: Some("/".to_string()),
					path_type: "Prefix".to_string(),
				}],
			}),
		})
		.collect();

	let kubernetes_ingress = Ingress {
		metadata: ObjectMeta {
			name: Some(format!("ingress-{}", deployment.id)),
			annotations: Some(annotations),
			..ObjectMeta::default()
		},
		spec: Some(IngressSpec {
			rules: Some(ingress_rules),
			..IngressSpec::default()
		}),
		..Ingress::default()
	};

	// Create the ingress defined above
	log::trace!("request_id: {} - creating ingress", request_id);
	let ingress_api =
		Api::<Ingress>::namespaced(kubernetes_client.clone(), namespace);

	ingress_api
		.patch(
			&format!("ingress-{}", deployment.id),
			&PatchParams::apply(&format!("ingress-{}", deployment.id)),
			&Patch::Apply(kubernetes_ingress),
		)
		.await?;

	log::trace!("request_id: {} - deployment created", request_id);

	log::trace!(
		"request_id: {} - App ingress is at {}",
		request_id,
		running_details
			.ports
			.iter()
			.filter(|(_, port_type)| *port_type == &ExposedPortType::Http)
			.map(|(port, _)| format!(
				"{}-{}.{}",
				port, deployment.id, config.cloudflare.onpatr_domain
			))
			.collect::<Vec<_>>()
			.join(",")
	);

	Ok(())
}

pub async fn delete_kubernetes_deployment(
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	kubeconfig: Kubeconfig,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client = super::get_kubernetes_client(kubeconfig).await?;

	Api::<K8sDeployment>::namespaced(
		kubernetes_client.clone(),
		workspace_id.as_str(),
	)
	.delete_opt(
		&format!("deployment-{}", deployment_id),
		&DeleteParams::default(),
	)
	.await?;

	log::trace!("request_id: {} - deleting the stateful set", request_id);

	Api::<StatefulSet>::namespaced(
		kubernetes_client.clone(),
		workspace_id.as_str(),
	)
	.delete_opt(&format!("sts-{}", deployment_id), &DeleteParams::default())
	.await?;

	log::trace!("request_id: {} - deleting the config map", request_id);

	Api::<ConfigMap>::namespaced(
		kubernetes_client.clone(),
		workspace_id.as_str(),
	)
	.delete_opt(
		&format!("config-mount-{}", deployment_id),
		&DeleteParams::default(),
	)
	.await?;

	log::trace!("request_id: {} - deleting the service", request_id);
	Api::<Service>::namespaced(
		kubernetes_client.clone(),
		workspace_id.as_str(),
	)
	.delete_opt(
		&format!("service-{}", deployment_id),
		&DeleteParams::default(),
	)
	.await?;

	log::trace!("request_id: {} - deleting the hpa", request_id);

	Api::<HorizontalPodAutoscaler>::namespaced(
		kubernetes_client.clone(),
		workspace_id.as_str(),
	)
	.delete_opt(&format!("hpa-{}", deployment_id), &DeleteParams::default())
	.await?;

	log::trace!("request_id: {} - deleting the ingress", request_id);
	Api::<Ingress>::namespaced(kubernetes_client, workspace_id.as_str())
		.delete_opt(
			&format!("ingress-{}", deployment_id),
			&DeleteParams::default(),
		)
		.await?;

	log::trace!(
		"request_id: {} - deployment deleted successfully!",
		request_id
	);

	Ok(())
}

pub async fn delete_kubernetes_volume(
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	volume: &DeploymentVolume,
	replica_index: u16,
	kubeconfig: Kubeconfig,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client = super::get_kubernetes_client(kubeconfig).await?;

	log::trace!("request_id: {} - deleting the pvc", request_id);
	Api::<PersistentVolumeClaim>::namespaced(
		kubernetes_client.clone(),
		workspace_id.as_str(),
	)
	.delete_opt(
		&format!(
			"pvc-{}-sts-{}-{}",
			volume.volume_id, deployment_id, replica_index
		),
		&DeleteParams::default(),
	)
	.await?;
	Ok(())
}

pub async fn update_kubernetes_volume(
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	volume: &DeploymentVolume,
	replica: u16,
	kubeconfig: Kubeconfig,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client = super::get_kubernetes_client(kubeconfig).await?;

	log::trace!("request_id: {} - updating the pvc", request_id);
	for idx in 0..replica {
		let storage_limit = [(
			"storage".to_string(),
			Quantity(format!("{}Gi", volume.size)),
		)]
		.into_iter()
		.collect::<BTreeMap<_, _>>();

		let kubernetes_pvc = PersistentVolumeClaim {
			metadata: ObjectMeta {
				name: Some(format!(
					"pvc-{}-sts-{}-{}",
					volume.volume_id, deployment_id, idx
				)),
				namespace: Some(workspace_id.to_string()),
				..ObjectMeta::default()
			},
			spec: Some(PersistentVolumeClaimSpec {
				access_modes: Some(vec!["ReadWriteOnce".to_string()]),
				resources: Some(ResourceRequirements {
					requests: Some(storage_limit),
					..ResourceRequirements::default()
				}),
				..PersistentVolumeClaimSpec::default()
			}),
			..PersistentVolumeClaim::default()
		};

		Api::<PersistentVolumeClaim>::namespaced(
			kubernetes_client.clone(),
			workspace_id.as_str(),
		)
		.patch(
			&format!("pvc-{}-sts-{}-{}", volume.volume_id, deployment_id, idx),
			&PatchParams::apply(&format!(
				"pvc-{}-sts-{}-{}",
				volume.volume_id, deployment_id, idx
			)),
			&Patch::Strategic(kubernetes_pvc),
		)
		.await?;
	}
	Ok(())
}

// TODO: add the logic of errored deployment
pub async fn get_kubernetes_deployment_status(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	namespace: &str,
) -> Result<DeploymentStatus, Error> {
	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let (kubeconfig, _) = service::get_kubernetes_config_for_region(
		connection,
		&deployment.region,
	)
	.await?;
	let kubernetes_client = super::get_kubernetes_client(kubeconfig).await?;

	let is_deployment =
		db::get_all_deployment_volumes(connection, deployment_id)
			.await?
			.is_empty();

	// if the ready replicas, is none then it is in deloying status.
	// we will update deployment status based on the number of replicas and
	// status message (CrashLoopBackOff/ErrImagePull)
	// see: https://develop.vicara.co/vicara/vicara-api/issues/807
	let container_status = if !is_deployment {
		Api::<StatefulSet>::namespaced(kubernetes_client.clone(), namespace)
			.get(&format!("sts-{}", deployment.id))
			.await
			.map(|status| status.status.and_then(|value| value.ready_replicas))
	} else {
		Api::<K8sDeployment>::namespaced(kubernetes_client.clone(), namespace)
			.get(&format!("deployment-{}", deployment.id))
			.await
			.map(|status| status.status.and_then(|value| value.ready_replicas))
	};

	let replicas = match container_status {
		Err(KubeError::Api(ErrorResponse { code: 404, .. })) => {
			// TODO: This is a temporary fix to solve issue #361.
			// Need to find a better solution to do this
			return Ok(DeploymentStatus::Errored);
		}
		Err(err) => return Err(err.into()),
		Ok(sts) => sts,
	};

	let deployment_status = if let Some(replicas) = replicas {
		if replicas >= deployment.min_horizontal_scale as i32 {
			DeploymentStatus::Running
		} else if replicas <= deployment.min_horizontal_scale as i32 {
			DeploymentStatus::Deploying
		} else {
			DeploymentStatus::Errored
		}
	} else {
		// if it is none, then it is not yet ready
		// might be in ContainerCreating state or in some errored state
		DeploymentStatus::Deploying
	};

	let event_api = Api::<Pod>::namespaced(kubernetes_client, namespace)
		.list(
			&ListParams::default()
				.labels(&format!("deploymentId={}", deployment_id)),
		)
		.await;

	let status = match event_api {
		Ok(event) => event
			.items
			.into_iter()
			.filter_map(|pod| pod.status)
			.filter_map(|status| status.container_statuses)
			.flat_map(|status| {
				status
					.into_iter()
					.filter_map(|status| status.state)
					.filter_map(|state| state.waiting)
					.filter_map(|waiting| waiting.reason)
					.collect::<Vec<_>>()
			})
			.collect::<Vec<_>>(),
		Err(err) => {
			log::trace!("Unable to get pod status, Error: {}", err);
			return Ok(DeploymentStatus::Deploying);
		}
	};

	// Better check handling and covering edge cases
	if status.contains(&"ErrImagePull".to_string()) ||
		status.contains(&"CrashLoopBackOff".to_string())
	{
		return Ok(DeploymentStatus::Errored);
	}
	Ok(deployment_status)
}
