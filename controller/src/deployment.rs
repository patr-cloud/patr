use std::{sync::Arc, time::Duration, collections::BTreeMap};

use futures::{future, FutureExt, StreamExt};
use k8s_openapi::{api::{
	apps::v1::{Deployment, StatefulSet},
	autoscaling::v2::HorizontalPodAutoscaler,
	core::v1::{ConfigMap, PersistentVolumeClaim, Service},
	networking::v1::Ingress,
}, apimachinery::pkg::api::resource::Quantity};
use kube::{
	core::ObjectMeta,
	runtime::{controller::Action, watcher, Controller},
	Api,
	Client, api::{PatchParams, Patch},
};
use tokio::signal;

use crate::{app::AppState, models::PatrDeployment};

/// Starts the deployment controller. This function will ideally run forever,
/// only exiting when a ctrl-c signal is received.
pub(super) async fn start_controller(client: Client, state: Arc<AppState>) {
	Controller::new(
		Api::<PatrDeployment>::all(client.clone()),
		watcher::Config::default(),
	)
	.owns(
		Api::<Deployment>::all(client.clone()),
		watcher::Config::default(),
	)
	.owns(
		Api::<StatefulSet>::all(client.clone()),
		watcher::Config::default(),
	)
	.owns(
		Api::<ConfigMap>::all(client.clone()),
		watcher::Config::default(),
	)
	.owns(
		Api::<Service>::all(client.clone()),
		watcher::Config::default(),
	)
	.owns(
		Api::<HorizontalPodAutoscaler>::all(client.clone()),
		watcher::Config::default(),
	)
	.owns(
		Api::<PersistentVolumeClaim>::all(client.clone()),
		watcher::Config::default(),
	)
	.owns(
		Api::<Ingress>::all(client.clone()),
		watcher::Config::default(),
	)
	.graceful_shutdown_on(signal::ctrl_c().map(|_| ()))
	.run(reconcile, error_policy, state)
	.for_each(|_| future::ready(()))
	.await;
}

/// Handles errors that occur during the reconciliation process. This function
/// is called whenever an error occurs during the reconciliation process. This
/// function should decide what to do with the error.
fn error_policy(_obj: Arc<PatrDeployment>, _err: &kube::Error, _ctx: Arc<AppState>) -> Action {
	Action::requeue(Duration::from_secs(5))
}

/// Reconciles the state of the cluster with the state of the Patr API. This
/// function is called whenever a new `PatrDeployment` is created, updated, or
/// deleted. In case a child object (an object owned by this controller) is
/// updated / deleted, this function is then as well. This function should
/// check with the Patr API to see if the deployment is up to date, and if not,
/// update it, along with all child objects.
async fn reconcile(deployment: Arc<PatrDeployment>, ctx: Arc<AppState>) -> Result<Action, kube::Error> {

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
				.map(|(path, data)| (path.to_string(), ByteString(data.clone().into())))
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
			if deployed_region.is_byoc_region() {
				config.vault.base_url()
			} else {
				config.vault.upstream_base_url()
			},
		),
		(
			"vault.security.banzaicloud.io/vault-skip-verify".to_string(),
			"false".to_string(),
		),
		(
			"vault.security.banzaicloud.io/vault-agent".to_string(),
			"false".to_string(),
		),
	]
	.into_iter()
	.chain(
		if deployed_region.is_byoc_region() {
			itertools::Either::Left([])
		} else {
			itertools::Either::Right([
				(
					"vault.security.banzaicloud.io/vault-role".to_string(),
					"vault".to_string(),
				),
				(
					"vault.security.banzaicloud.io/vault-path".to_string(),
					"kubernetes".to_string(),
				),
			])
		}
		.into_iter(),
	)
	.collect();

	if !deployment_volumes.is_empty() {
		// Deleting STS using orphan flag to update the volumes
		log::trace!(
			"request_id: {} deleting sts without deleting the pod",
			request_id
		);
		Api::<StatefulSet>::namespaced(kubernetes_client.clone(), workspace_id.as_str())
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
						target_port: Some(IntOrString::Int(port.value() as i32)),
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
	let service_api = Api::<Service>::namespaced(kubernetes_client.clone(), namespace);

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
				startup_probe: running_details.startup_probe.as_ref().map(|probe| Probe {
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
				}),
				liveness_probe: running_details.liveness_probe.as_ref().map(|probe| Probe {
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
						.chain(
							if deployed_region.is_byoc_region() {
								itertools::Either::Left([
									EnvVar {
										name: "VAULT_AUTH_METHOD".to_string(),
										value: Some("token".to_string()),
										..EnvVar::default()
									},
									EnvVar {
										name: "VAULT_TOKEN".to_string(),
										value_from: Some(EnvVarSource {
											secret_key_ref: Some(SecretKeySelector {
												name: Some(PATR_BYOC_TOKEN_NAME.to_string()),
												key: PATR_BYOC_TOKEN_VALUE_NAME.to_string(),
												..Default::default()
											}),
											..Default::default()
										}),
										..Default::default()
									},
								])
							} else {
								itertools::Either::Right([])
							}
							.into_iter(),
						)
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
			image_pull_secrets: deployment.registry.is_patr_registry().then(|| {
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
						max_unavailable: Some(IntOrString::String("25%".to_owned())),
					}),
				}),
				..DeploymentSpec::default()
			}),
			..K8sDeployment::default()
		};

		// Create the deployment defined above
		log::trace!("request_id: {} - creating deployment", request_id);
		let deployment_api = Api::<K8sDeployment>::namespaced(kubernetes_client.clone(), namespace);

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

		Api::<StatefulSet>::namespaced(kubernetes_client.clone(), workspace_id.as_str())
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
		let hpa_api =
			Api::<HorizontalPodAutoscaler>::namespaced(kubernetes_client.clone(), namespace);

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
		let sts_api = Api::<StatefulSet>::namespaced(kubernetes_client.clone(), namespace);

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

		Api::<K8sDeployment>::namespaced(kubernetes_client.clone(), workspace_id.as_str())
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
		let hpa_api =
			Api::<HorizontalPodAutoscaler>::namespaced(kubernetes_client.clone(), namespace);

		hpa_api
			.delete_opt(&format!("hpa-{}", deployment.id), &DeleteParams::default())
			.await?;
	}

	// For a deployment has more than one replica, then only we can use
	// pod-disruption-budget to move pods between nodes without any down time.
	// Even with hpa of max=4 but min=1 if the number of pods currently running
	// is 1, then it will block the node drain
	if running_details.min_horizontal_scale > 1 {
		// Create pdb for deployment alone
		// For sts, we can't use pdb as it involves state handling
		// see: https://kubernetes.io/docs/tasks/run-application/configure-pdb/#think-about-how-your-application-reacts-to-disruptions
		log::trace!(
			"request_id: {} - creating pod disruption budget",
			request_id
		);

		let pdb = PodDisruptionBudget {
			metadata: ObjectMeta {
				name: Some(format!("pdb-{}", deployment.id)),
				namespace: Some(namespace.to_string()),
				labels: Some(labels.clone()),
				..Default::default()
			},
			spec: Some(PodDisruptionBudgetSpec {
				selector: Some(LabelSelector {
					match_labels: Some(labels.clone()),
					..Default::default()
				}),
				min_available: Some(IntOrString::String("50%".to_owned())),
				..Default::default()
			}),
			..Default::default()
		};

		Api::<PodDisruptionBudget>::namespaced(kubernetes_client.clone(), namespace)
			.patch(
				&format!("pdb-{}", deployment.id),
				&PatchParams::apply(&format!("pdb-{}", deployment.id)),
				&Patch::Apply(pdb),
			)
			.await?;
	} else {
		log::trace!(
			"request_id: {} - min replica is not more than one, so deleting the pdb (if present)",
			request_id
		);

		Api::<PodDisruptionBudget>::namespaced(kubernetes_client.clone(), namespace)
			.delete_opt(&format!("pdb-{}", deployment.id), &DeleteParams::default())
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
				port, deployment.id, deployed_region.id, config.cloudflare.onpatr_domain
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
	let ingress_api = Api::<Ingress>::namespaced(kubernetes_client.clone(), namespace);

	ingress_api
		.patch(
			&format!("ingress-{}", deployment.id),
			&PatchParams::apply(&format!("ingress-{}", deployment.id)),
			&Patch::Apply(kubernetes_ingress),
		)
		.await?;

	Ok(Action::requeue(Duration::from_secs(3600)))
}
