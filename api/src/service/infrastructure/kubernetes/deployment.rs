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
			IngressTLS,
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
	api::{DeleteParams, ListParams, Patch, PatchParams},
	core::{ErrorResponse, ObjectMeta},
	Api,
	Error as KubeError,
};
use sha2::{Digest, Sha512};

use crate::{
	db::{self, DeploymentVolume},
	error,
	models::deployment,
	service::{
		self,
		ext_traits::DeleteOpt,
		get_kubernetes_config_for_default_region,
		ClusterType,
		KubernetesConfigDetails,
	},
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
	kubeconfig: KubernetesConfigDetails,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let cluster_type = kubeconfig.cluster_type.clone();
	let kubernetes_client =
		super::get_kubernetes_client(kubeconfig.auth_details.clone()).await?;

	// the namespace is workspace id
	let namespace = workspace_id.as_str();

	log::trace!(
		"Deploying the container with id: {} on kubernetes with request_id: {}",
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
			config.vault.address.clone(),
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

	let mut volume_mounts: Vec<VolumeMount> = Vec::new();
	let mut volumes: Vec<Volume> = Vec::new();
	let mut pvc: Vec<PersistentVolumeClaim> = Vec::new();

	if !&running_details.config_mounts.is_empty() {
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
			cluster_ip: if running_details.volume.is_empty() {
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

	if running_details.volume.is_empty() {
		let kubernetes_deployment = K8sDeployment {
			metadata: ObjectMeta {
				name: Some(format!("deployment-{}", deployment.id)),
				namespace: Some(namespace.to_string()),
				labels: Some(labels.clone()),
				..ObjectMeta::default()
			},
			spec: Some(DeploymentSpec {
				replicas: Some(running_details.min_horizontal_scale as i32),
				selector: LabelSelector {
					match_expressions: None,
					match_labels: Some(labels.clone()),
				},
				template: PodTemplateSpec {
					spec: Some(PodSpec {
						containers: vec![Container {
							name: format!("deployment-{}", deployment.id),
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
							startup_probe: running_details
								.startup_probe
								.as_ref()
								.map(|probe| Probe {
									http_get: Some(HTTPGetAction {
										path: Some(probe.path.clone()),
										port: IntOrString::Int(
											probe.port as i32,
										),
										scheme: Some("HTTP".to_string()),
										..HTTPGetAction::default()
									}),
									failure_threshold: Some(15),
									period_seconds: Some(10),
									timeout_seconds: Some(3),
									..Probe::default()
								}),
							liveness_probe: running_details
								.liveness_probe
								.as_ref()
								.map(|probe| Probe {
									http_get: Some(HTTPGetAction {
										path: Some(probe.path.clone()),
										port: IntOrString::Int(
											probe.port as i32,
										),
										scheme: Some("HTTP".to_string()),
										..HTTPGetAction::default()
									}),
									failure_threshold: Some(15),
									period_seconds: Some(10),
									timeout_seconds: Some(3),
									..Probe::default()
								}),
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
											value: Some(
												workspace_id.to_string(),
											),
											..EnvVar::default()
										},
										EnvVar {
											name: "DEPLOYMENT_ID".to_string(),
											value: Some(
												deployment.id.to_string(),
											),
											..EnvVar::default()
										},
										EnvVar {
											name: "DEPLOYMENT_NAME".to_string(),
											value: Some(
												deployment.name.clone(),
											),
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
										(
											"memory".to_string(),
											Quantity("25M".to_owned()),
										),
										(
											"cpu".to_string(),
											Quantity("50m".to_owned()),
										),
									]
									.into_iter()
									.collect(),
								),
							}),
							volume_mounts: if !running_details
								.config_mounts
								.is_empty()
							{
								Some(vec![VolumeMount {
									name: "config-mounts".to_string(),
									mount_path: "/etc/config".to_string(),
									..VolumeMount::default()
								}])
							} else {
								None
							},
							..Container::default()
						}],
						volumes: if !running_details.config_mounts.is_empty() {
							Some(vec![Volume {
								name: "config-mounts".to_string(),
								config_map: Some(ConfigMapVolumeSource {
									name: Some(format!(
										"config-mount-{}",
										deployment.id
									)),
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
							}])
						} else {
							None
						},
						image_pull_secrets: deployment
							.registry
							.is_patr_registry()
							.then(|| {
								// TODO: for now patr registry is not supported
								// for user clusters, need to create a separate
								// secret for each private repo in future
								vec![LocalObjectReference {
									name: Some("patr-regcred".to_string()),
								}]
							}),
						..PodSpec::default()
					}),
					metadata: Some(ObjectMeta {
						labels: Some(labels.clone()),
						annotations: Some(annotations),
						..ObjectMeta::default()
					}),
				},
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
	} else {
		let kubernetes_sts = StatefulSet {
			metadata: ObjectMeta {
				name: Some(format!("sts-{}", deployment.id)),
				namespace: Some(workspace_id.to_string()),
				labels: Some(labels.clone()),
				..ObjectMeta::default()
			},
			spec: Some(StatefulSetSpec {
				replicas: Some(running_details.min_horizontal_scale as i32),
				selector: LabelSelector {
					match_expressions: None,
					match_labels: Some(labels.clone()),
				},
				service_name: format!("service-{}", deployment.id),
				template: PodTemplateSpec {
					spec: Some(PodSpec {
						containers: vec![Container {
							name: format!("sts-{}", deployment.id),
							image: Some(image_name.to_string()),
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
							startup_probe: running_details
								.startup_probe
								.as_ref()
								.map(|probe| Probe {
									http_get: Some(HTTPGetAction {
										path: Some(probe.path.clone()),
										port: IntOrString::Int(
											probe.port as i32,
										),
										scheme: Some("HTTP".to_string()),
										..HTTPGetAction::default()
									}),
									failure_threshold: Some(15),
									period_seconds: Some(10),
									timeout_seconds: Some(3),
									..Probe::default()
								}),
							liveness_probe: running_details
								.startup_probe
								.as_ref()
								.map(|probe| Probe {
									http_get: Some(HTTPGetAction {
										path: Some(probe.path.clone()),
										port: IntOrString::Int(
											probe.port as i32,
										),
										scheme: Some("HTTP".to_string()),
										..HTTPGetAction::default()
									}),
									failure_threshold: Some(15),
									period_seconds: Some(10),
									timeout_seconds: Some(3),
									..Probe::default()
								}),
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
											value: Some(
												workspace_id.to_string(),
											),
											..EnvVar::default()
										},
										EnvVar {
											name: "DEPLOYMENT_ID".to_string(),
											value: Some(
												deployment.id.to_string(),
											),
											..EnvVar::default()
										},
										EnvVar {
											name: "DEPLOYMENT_NAME".to_string(),
											value: Some(
												deployment.name.clone(),
											),
											..EnvVar::default()
										},
										EnvVar {
											name: "CONFIG_MAP_HASH".to_string(),
											value: Some(
												config_map_hash.to_string(),
											),
											..EnvVar::default()
										},
									])
									.collect::<Vec<_>>(),
							),
							resources: Some(ResourceRequirements {
								limits: Some(machine_type.clone()),
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
										(
											"memory".to_string(),
											Quantity("25M".to_owned()),
										),
										(
											"cpu".to_string(),
											Quantity("50m".to_owned()),
										),
									]
									.into_iter()
									.collect(),
								),
							}),
							volume_mounts: Some(volume_mounts.to_vec()),
							..Container::default()
						}],
						volumes: Some(volumes),

						image_pull_secrets: deployment
							.registry
							.is_patr_registry()
							.then(|| {
								// TODO: for now patr registry is not supported
								// for user clusters, need to create a separate
								// secret for each private repo in future
								vec![LocalObjectReference {
									name: Some("patr-regcred".to_string()),
								}]
							}),
						..PodSpec::default()
					}),
					metadata: Some(ObjectMeta {
						labels: Some(labels.clone()),
						annotations: Some(annotations),
						..ObjectMeta::default()
					}),
				},
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
	}

	let annotations = [
		(
			"kubernetes.io/ingress.class".to_string(),
			"nginx".to_string(),
		),
		(
			"cert-manager.io/cluster-issuer".to_string(),
			config.kubernetes.cert_issuer_dns.clone(),
		),
	]
	.into_iter()
	.collect::<BTreeMap<_, _>>();

	let (default_ingress_rules, default_tls_rules) = running_details
		.ports
		.iter()
		.filter(|(_, port_type)| *port_type == &ExposedPortType::Http)
		.map(|(port, _)| {
			(
				IngressRule {
					host: Some(format!(
						"{}-{}.patr.cloud",
						port, deployment.id
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
							path_type: Some("Prefix".to_string()),
						}],
					}),
				},
				IngressTLS {
					hosts: Some(vec![
						"*.patr.cloud".to_string(),
						"patr.cloud".to_string(),
					]),
					secret_name: None,
				},
			)
		})
		.unzip::<_, _, Vec<_>, Vec<_>>();

	let kubernetes_ingress = Ingress {
		metadata: ObjectMeta {
			name: Some(format!("ingress-{}", deployment.id)),
			annotations: Some(annotations.clone()),
			..ObjectMeta::default()
		},
		spec: Some(IngressSpec {
			rules: Some(default_ingress_rules),
			tls: Some(default_tls_rules.clone()),
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

	if let ClusterType::UserOwned {
		region_id,
		ingress_ip_addr: _,
	} = cluster_type
	{
		// create a ingress in patr cluster to point to user's cluster
		let kubernetes_client = super::get_kubernetes_client(
			get_kubernetes_config_for_default_region(config).auth_details,
		)
		.await?;

		let exposted_ports = running_details
			.ports
			.iter()
			.filter(|(_, port_type)| *port_type == &ExposedPortType::Http);

		for (port, _) in exposted_ports {
			let ingress_rule = IngressRule {
				host: Some(format!("{}-{}.patr.cloud", port, deployment.id)),
				http: Some(HTTPIngressRuleValue {
					paths: vec![HTTPIngressPath {
						backend: IngressBackend {
							service: Some(IngressServiceBackend {
								name: format!("service-{}", region_id.as_str()),
								port: Some(ServiceBackendPort {
									number: Some(80),
									..ServiceBackendPort::default()
								}),
							}),
							..Default::default()
						},
						path: Some("/".to_string()),
						path_type: Some("Prefix".to_string()),
					}],
				}),
			};

			let annotations = [
				(
					"kubernetes.io/ingress.class".to_string(),
					"nginx".to_string(),
				),
				(
					String::from("nginx.ingress.kubernetes.io/upstream-vhost"),
					format!("{}-{}.patr.cloud", port, deployment.id),
				),
				(
					String::from(
						"nginx.ingress.kubernetes.io/backend-protocol",
					),
					"HTTP".to_string(),
				),
				// TODO: add cert manager annotations,
				// once ssl certificate is used in customers cluster
			]
			.into_iter()
			.collect();

			let kubernetes_ingress = Ingress {
				metadata: ObjectMeta {
					name: Some(format!("ingress-{}", deployment.id)),
					annotations: Some(annotations),
					..ObjectMeta::default()
				},
				spec: Some(IngressSpec {
					rules: Some(vec![ingress_rule]),
					tls: Some(vec![IngressTLS {
						hosts: Some(vec![
							"*.patr.cloud".to_string(),
							"patr.cloud".to_string(),
						]),
						secret_name: None,
					}]),
					..IngressSpec::default()
				}),
				..Ingress::default()
			};

			let ingress_api = Api::<Ingress>::namespaced(
				kubernetes_client.clone(),
				namespace,
			);

			ingress_api
				.patch(
					&format!("ingress-{}", deployment.id),
					&PatchParams::apply(&format!("ingress-{}", deployment.id)),
					&Patch::Apply(kubernetes_ingress),
				)
				.await?;
		}
	}

	log::trace!("request_id: {} - deployment created", request_id);

	log::trace!(
		"request_id: {} - App ingress is at {}",
		request_id,
		running_details
			.ports
			.iter()
			.filter(|(_, port_type)| *port_type == &ExposedPortType::Http)
			.map(|(port, _)| format!("{}-{}.patr.cloud", port, deployment.id))
			.collect::<Vec<_>>()
			.join(",")
	);

	Ok(())
}

pub async fn delete_kubernetes_deployment(
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	kubeconfig: KubernetesConfigDetails,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client =
		super::get_kubernetes_client(kubeconfig.auth_details).await?;

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

pub async fn delete_deployment_volume(
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	volumes: &Vec<DeploymentVolume>,
	replica_range: (u16, u16),
	kubeconfig: KubernetesConfigDetails,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client =
		super::get_kubernetes_client(kubeconfig.auth_details).await?;

	log::trace!("request_id: {} - deleting the pvc", request_id);
	for volume in volumes {
		for idx in replica_range.0..replica_range.1 {
			Api::<PersistentVolumeClaim>::namespaced(
				kubernetes_client.clone(),
				workspace_id.as_str(),
			)
			.delete_opt(
				&format!(
					"pvc-{}-sts-{}-{}",
					volume.volume_id, deployment_id, idx
				),
				&DeleteParams::default(),
			)
			.await?;
		}
	}
	Ok(())
}

// TODO: add the logic of errored deployment
pub async fn get_kubernetes_deployment_status(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	namespace: &str,
	config: &Settings,
) -> Result<DeploymentStatus, Error> {
	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let kubeconfig = service::get_kubernetes_config_for_region(
		connection,
		&deployment.region,
		config,
	)
	.await?;
	let kubernetes_client =
		super::get_kubernetes_client(kubeconfig.auth_details).await?;

	let is_deployment =
		db::get_all_deployment_volumes(connection, deployment_id)
			.await?
			.is_empty();

	let container_status = if !is_deployment {
		Api::<StatefulSet>::namespaced(kubernetes_client.clone(), namespace)
			.get(&format!("sts-{}", deployment.id))
			.await
			.map(|status| {
				status.status.and_then(|value| value.current_replicas)
			})
	} else {
		Api::<K8sDeployment>::namespaced(kubernetes_client.clone(), namespace)
			.get(&format!("deployment-{}", deployment.id))
			.await
			.map(|status| {
				status.status.and_then(|value| value.available_replicas)
			})
	};

	let replicas = match container_status {
		Err(KubeError::Api(ErrorResponse { code: 404, .. })) => {
			// TODO: This is a temporary fix to solve issue #361.
			// Need to find a better solution to do this
			return Ok(DeploymentStatus::Deploying);
		}
		Err(err) => return Err(err.into()),
		Ok(sts) => sts.status(500).body(error!(SERVER_ERROR).to_string())?,
	};

	let deployment_status =
		if replicas == deployment.min_horizontal_scale as i32 {
			DeploymentStatus::Running
		} else if replicas <= deployment.min_horizontal_scale as i32 {
			DeploymentStatus::Deploying
		} else {
			DeploymentStatus::Errored
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
