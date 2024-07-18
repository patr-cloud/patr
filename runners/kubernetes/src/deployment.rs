use std::{collections::BTreeMap, str::FromStr, sync::Arc, time::Duration};

use futures::{future, StreamExt};
use k8s_openapi::{
	api::{
		apps::v1::{
			Deployment as KubeDeployment,
			DeploymentSpec,
			DeploymentStrategy,
			RollingUpdateDeployment,
			StatefulSet,
			StatefulSetSpec,
			StatefulSetUpdateStrategy,
		},
		autoscaling::v1::*,
		core::v1::*,
		networking::v1::*,
		policy::v1::*,
	},
	apimachinery::pkg::{
		api::resource::Quantity,
		apis::meta::v1::LabelSelector,
		util::intstr::IntOrString,
	},
	ByteString,
};
use kube::{
	api::{DeleteParams, Patch, PatchParams, PropagationPolicy, Resource},
	core::ObjectMeta,
	runtime::{
		controller::{Action, Controller},
		reflector::ObjectRef,
		watcher,
	},
	Api,
	Client,
};
use models::{
	api::workspace::{container_registry::*, deployment::*, volume::*},
	prelude::*,
};
use sha2::{Digest, Sha512};
use tokio::{
	sync::{
		broadcast::Receiver,
		mpsc::{self, UnboundedReceiver, UnboundedSender},
	},
	task,
	task::JoinHandle,
};
use tokio_stream::wrappers::{BroadcastStream, UnboundedReceiverStream};

use crate::{client::make_request, constants, prelude::*};

/// Starts the deployment controller. This function will spawn a new task that
/// will run the controller. This function will return a sender that can be
/// used to send a message to the controller to reconcile all deployments.
pub(super) fn start_controller(
	client: Client,
	state: Arc<AppState>,
	patr_update_sender: Receiver<()>,
) -> (UnboundedSender<()>, JoinHandle<()>) {
	let (sender, receiver) = mpsc::unbounded_channel::<()>();
	let handle = task::spawn(run_controller(client, state, receiver, patr_update_sender));
	(sender, handle)
}

/// This function will ideally run forever, only exiting when a ctrl-c signal is
/// received.
async fn run_controller(
	client: Client,
	state: Arc<AppState>,
	reconcile_receiver: UnboundedReceiver<()>,
	patr_update_sender: Receiver<()>,
) {
	Controller::new(
		Api::<PatrDeployment>::all(client.clone()),
		watcher::Config::default(),
	)
	.owns(
		Api::<KubeDeployment>::all(client.clone()),
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
	.reconcile_all_on(UnboundedReceiverStream::new(reconcile_receiver))
	.reconcile_on(BroadcastStream::new(patr_update_sender).filter_map(
		|_input: Result<(), tokio_stream::wrappers::errors::BroadcastStreamRecvError>| async move {
			Some(ObjectRef::new(&format!("deployment-{}", Uuid::nil())).within("default"))
		},
	))
	.graceful_shutdown_on(crate::exit_signal())
	.run(reconcile, error_policy, state)
	.for_each(|_| future::ready(()))
	.await;
}

/// Handles errors that occur during the reconciliation process. This function
/// is called whenever an error occurs during the reconciliation process. This
/// function should decide what to do with the error.
#[instrument(skip(_ctx))]
fn error_policy(_obj: Arc<PatrDeployment>, _err: &AppError, _ctx: Arc<AppState>) -> Action {
	Action::requeue(Duration::from_secs(5))
}

/// Reconciles the state of the cluster with the state of the Patr API. This
/// function is called whenever a new `PatrDeployment` is created, updated, or
/// deleted. In case a child object (an object owned by this controller) is
/// updated / deleted, this function is then as well. This function should
/// check with the Patr API to see if the deployment is up to date, and if not,
/// update it, along with all child objects.
#[instrument(skip(ctx))]
async fn reconcile(
	deployment_object: Arc<PatrDeployment>,
	ctx: Arc<AppState>,
) -> Result<Action, AppError> {
	info!(
		"Reconciling deployment with id: {}",
		deployment_object.spec.deployment.id
	);

	let namespace = deployment_object
		.metadata
		.namespace
		.as_deref()
		.ok_or_else(|| {
			AppError::InternalError(
				"The provided PatrDeployment does not have a namespace".to_string(),
			)
		})?;
	let owner_reference = deployment_object.controller_owner_ref(&()).ok_or_else(|| {
		AppError::InternalError("Deployment does not have a controller owner reference".to_string())
	})?;

	let spec = &deployment_object.spec;

	trace!("Computing hash of config map data");
	let config_map_hash = hex::encode(
		spec.running_details
			.config_mounts
			.iter()
			.fold(Sha512::default(), |acc, (key, value)| {
				acc.chain_update(key).chain_update(value)
			})
			.finalize(),
	);
	trace!(
		"Patching ConfigMap for deployment with id: {}",
		spec.deployment.id
	);

	Api::<ConfigMap>::namespaced(ctx.client.clone(), namespace)
		.patch(
			&format!("config-mount-{}", spec.deployment.id),
			&PatchParams::apply(&format!("config-mount-{}", spec.deployment.id)),
			&Patch::Apply(ConfigMap {
				metadata: ObjectMeta {
					name: Some(format!("config-mount-{}", spec.deployment.id)),
					owner_references: Some(vec![owner_reference.clone()]),
					..ObjectMeta::default()
				},
				binary_data: Some(
					spec.running_details
						.config_mounts
						.iter()
						.map(|(path, data)| (path.to_string(), ByteString(data.clone().into())))
						.collect(),
				),
				..ConfigMap::default()
			}),
		)
		.await?;

	let machine_type = make_request(
		ApiRequest::<ListAllDeploymentMachineTypeRequest>::builder()
			.path(ListAllDeploymentMachineTypePath {
				workspace_id: ctx.workspace_id,
			})
			.headers(ListAllDeploymentMachineTypeRequestHeaders {
				user_agent: UserAgent::from_static("deployment-controller"),
			})
			.query(())
			.body(ListAllDeploymentMachineTypeRequest)
			.build(),
	)
	.await
	.map_err(|err| err.body.error)?
	.body
	.machine_types
	.into_iter()
	.find(|machine_type| machine_type.id == spec.deployment.machine_type)
	.ok_or_else(|| AppError::InternalError("invalid machine type".to_string()))?;

	trace!("Deploying deployment: {}", spec.deployment.id);

	let labels = [
		(
			constants::DEPLOYMENT_ID.to_string(),
			spec.deployment.id.to_string(),
		),
		(constants::WORKSPACE_ID.to_string(), namespace.to_string()),
		(
			constants::RUNNER.to_string(),
			spec.deployment.runner.to_string(),
		),
		("app.kubernetes.io/name".to_string(), "vault".to_string()),
	]
	.into_iter()
	.collect::<BTreeMap<_, _>>();

	trace!("generating deployment configuration");

	if !spec.running_details.volumes.is_empty() {
		// Deleting stateful set using orphan flag to update the volumes
		trace!("deleting stateful set without deleting the pod");
		Api::<StatefulSet>::namespaced(ctx.client.clone(), namespace)
			.delete(
				&format!("sts-{}", spec.deployment.id),
				&DeleteParams {
					propagation_policy: Some(PropagationPolicy::Orphan),
					..DeleteParams::default()
				},
			)
			.await?;
	}

	let mut volume_mounts = Vec::new();
	let mut volumes = Vec::new();
	let mut pvc = Vec::new();

	if !spec.running_details.config_mounts.is_empty() {
		volume_mounts.push(VolumeMount {
			name: "config-mounts".to_string(),
			mount_path: "/etc/config".to_string(),
			..VolumeMount::default()
		});
		volumes.push(Volume {
			name: "config-mounts".to_string(),
			config_map: Some(ConfigMapVolumeSource {
				name: Some(format!("config-mount-{}", spec.deployment.id)),
				items: Some(
					spec.running_details
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
		});
	}

	for (&volume_id, mount_path) in &spec.running_details.volumes {
		let volume = make_request(
			ApiRequest::<GetVolumeInfoRequest>::builder()
				.path(GetVolumeInfoPath {
					workspace_id: ctx.workspace_id,
					volume_id,
				})
				.headers(GetVolumeInfoRequestHeaders {
					authorization: BearerToken::from_str(&ctx.patr_token).map_err(|err| {
						ErrorType::server_error(format!("invalid patr token. Error: `{}`", err))
					})?,
					user_agent: UserAgent::from_static("deployment-controller"),
				})
				.query(())
				.body(GetVolumeInfoRequest)
				.build(),
		)
		.await
		.map_err(|err| err.body.error)?
		.body
		.volume;

		volume_mounts.push(VolumeMount {
			name: format!("pvc-{}", volume_id),
			// make sure user does not have the mount_path in the directory
			// in the fs, by my observation it gives crashLoopBackOff error
			mount_path: mount_path.clone(),
			..VolumeMount::default()
		});

		pvc.push(PersistentVolumeClaim {
			metadata: ObjectMeta {
				name: Some(format!("pvc-{}", volume_id)),
				namespace: Some(namespace.to_string()),
				owner_references: Some(vec![owner_reference.clone()]),
				..ObjectMeta::default()
			},
			spec: Some(PersistentVolumeClaimSpec {
				access_modes: Some(vec!["ReadWriteOnce".to_string()]),
				resources: Some(VolumeResourceRequirements {
					requests: Some(
						[(
							"storage".to_string(),
							Quantity(format!("{}Gi", volume.size)),
						)]
						.into(),
					),
					..VolumeResourceRequirements::default()
				}),
				..PersistentVolumeClaimSpec::default()
			}),
			..PersistentVolumeClaim::default()
		});
	}

	trace!("Creating deployment service");

	Api::<Service>::namespaced(ctx.client.clone(), namespace)
		.patch(
			&format!("service-{}", spec.deployment.id),
			&PatchParams::apply(&format!("service-{}", spec.deployment.id)),
			&Patch::Apply(Service {
				metadata: ObjectMeta {
					name: Some(format!("service-{}", spec.deployment.id)),
					owner_references: Some(vec![owner_reference.clone()]),
					..ObjectMeta::default()
				},
				spec: Some(ServiceSpec {
					ports: Some(
						spec.running_details
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
					cluster_ip: if spec.running_details.volumes.is_empty() {
						None
					} else {
						Some("None".to_string())
					},
					..ServiceSpec::default()
				}),
				..Service::default()
			}),
		)
		.await?;

	let image_name = match &spec.deployment.registry {
		DeploymentRegistry::PatrRegistry {
			registry,
			repository_id,
		} => {
			let repository = make_request(
				ApiRequest::<GetContainerRepositoryInfoRequest>::builder()
					.path(GetContainerRepositoryInfoPath {
						workspace_id: Uuid::parse_str(namespace).unwrap(),
						repository_id: *repository_id,
					})
					.headers(GetContainerRepositoryInfoRequestHeaders {
						authorization: BearerToken::from_str(&ctx.patr_token).map_err(|err| {
							ErrorType::server_error(format!("invalid patr token. Error: `{}`", err))
						})?,
						user_agent: UserAgent::from_static("deployment-controller"),
					})
					.query(())
					.body(GetContainerRepositoryInfoRequest)
					.build(),
			)
			.await
			.map_err(|err| err.body.error)?
			.body
			.repository;

			format!("{}/{}", registry, repository.name)
		}
		DeploymentRegistry::ExternalRegistry {
			registry,
			image_name,
		} => format!("{}/{}", registry, image_name),
	};

	let image_name = if let Some(current_live_digest) = &spec.deployment.current_live_digest {
		format!("{}@{}", image_name, current_live_digest)
	} else {
		format!("{}:{}", image_name, spec.deployment.image_tag)
	};

	let metadata = ObjectMeta {
		name: Some(format!(
			"{}-{}",
			if spec.running_details.volumes.is_empty() {
				"deployment"
			} else {
				"sts"
			},
			spec.deployment.id
		)),
		namespace: Some(namespace.to_string()),
		labels: Some(labels.clone()),
		owner_references: Some(vec![owner_reference.clone()]),
		..ObjectMeta::default()
	};
	let replicas = Some(spec.running_details.min_horizontal_scale.into());
	let selector = LabelSelector {
		match_expressions: None,
		match_labels: Some(labels.clone()),
	};
	let template =
		PodTemplateSpec {
			spec: Some(PodSpec {
				containers: vec![Container {
					name: format!(
						"{}-{}",
						if spec.running_details.volumes.is_empty() {
							"deployment"
						} else {
							"sts"
						},
						spec.deployment.id
					),
					image: Some(image_name),
					image_pull_policy: Some("Always".to_string()),
					ports: Some(
						spec.running_details
							.ports
							.keys()
							.map(|port| ContainerPort {
								container_port: port.value().into(),
								..ContainerPort::default()
							})
							.collect::<Vec<_>>(),
					),
					startup_probe: spec
						.running_details
						.startup_probe
						.as_ref()
						.map(|probe| Probe {
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
					liveness_probe: spec.running_details.liveness_probe.as_ref().map(|probe| {
						Probe {
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
						}
					}),
					env: Some(
						spec.running_details
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
												namespace, from_secret
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
									value: Some(namespace.to_string()),
									..EnvVar::default()
								},
								EnvVar {
									name: "DEPLOYMENT_ID".to_string(),
									value: Some(spec.deployment.id.to_string()),
									..EnvVar::default()
								},
								EnvVar {
									name: "DEPLOYMENT_NAME".to_string(),
									value: Some(spec.deployment.name.clone()),
									..EnvVar::default()
								},
								EnvVar {
									name: "CONFIG_MAP_HASH".to_string(),
									value: Some(config_map_hash),
									..EnvVar::default()
								},
								EnvVar {
									name: "VAULT_AUTH_METHOD".to_string(),
									value: Some("token".to_string()),
									..EnvVar::default()
								},
								EnvVar {
									name: "VAULT_TOKEN".to_string(),
									value: Some(ctx.patr_token.clone()),
									..Default::default()
								},
							])
							.collect::<Vec<_>>(),
					),
					resources: Some(ResourceRequirements {
						limits: Some(
							[
								(
									"memory".to_string(),
									Quantity(format!(
										"{:.1}G",
										(machine_type.memory_count as f64) / 4f64
									)),
								),
								(
									"cpu".to_string(),
									Quantity(format!("{:.1}", machine_type.cpu_count as f64)),
								),
							]
							.into(),
						),
						// https://blog.kubecost.com/blog/requests-and-limits/#the-tradeoffs
						// using too low values for resource request
						// will result in frequent pod restarts if
						// memory usage increases and may result in
						// starvation
						//
						// currently used 5% of the minimum deployment
						// machine type as a request values
						requests: Some(
							[
								("memory".to_string(), Quantity("25M".to_owned())),
								("cpu".to_string(), Quantity("50m".to_owned())),
							]
							.into_iter()
							.collect(),
						),
						claims: None,
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
				image_pull_secrets: spec.deployment.registry.is_patr_registry().then(|| {
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
				annotations: Some(
					[
						(
							"vault.security.banzaicloud.io/vault-addr".to_string(),
							"https://secrets.patr.cloud".to_string(),
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
							"vault.security.banzaicloud.io/vault-role".to_string(),
							"vault".to_string(),
						),
						(
							"vault.security.banzaicloud.io/vault-path".to_string(),
							"kubernetes".to_string(),
						),
					]
					.into(),
				),
				owner_references: Some(vec![owner_reference.clone()]),
				..ObjectMeta::default()
			}),
		};

	if spec.running_details.volumes.is_empty() {
		let kubernetes_deployment = KubeDeployment {
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
			..KubeDeployment::default()
		};

		// Create the deployment defined above
		trace!("creating deployment");
		let deployment_api = Api::<KubeDeployment>::namespaced(ctx.client.clone(), namespace);

		deployment_api
			.patch(
				&format!("deployment-{}", spec.deployment.id),
				&PatchParams::apply(&format!("deployment-{}", spec.deployment.id)),
				&Patch::Apply(kubernetes_deployment),
			)
			.await?;

		// This is because if a user wanted to delete the volume from there sts
		// then a sts will be converted to deployment
		trace!("deleting the stateful set if there are any");

		Api::<StatefulSet>::namespaced(ctx.client.clone(), namespace)
			.delete_opt(
				&format!("sts-{}", spec.deployment.id),
				&DeleteParams::default(),
			)
			.await?;

		// HPA - horizontal pod autoscaler
		let kubernetes_hpa = HorizontalPodAutoscaler {
			metadata: ObjectMeta {
				name: Some(format!("hpa-{}", spec.deployment.id)),
				namespace: Some(namespace.to_string()),
				owner_references: Some(vec![owner_reference.clone()]),
				..ObjectMeta::default()
			},
			spec: Some(HorizontalPodAutoscalerSpec {
				scale_target_ref: CrossVersionObjectReference {
					api_version: Some("apps/v1".to_string()),
					kind: "Deployment".to_string(),
					name: format!("deployment-{}", spec.deployment.id),
				},
				min_replicas: Some(spec.running_details.min_horizontal_scale.into()),
				max_replicas: spec.running_details.max_horizontal_scale.into(),
				target_cpu_utilization_percentage: Some(80),
			}),
			..HorizontalPodAutoscaler::default()
		};

		// Create the HPA defined above
		trace!("creating horizontal pod autoscaler");
		let hpa_api = Api::<HorizontalPodAutoscaler>::namespaced(ctx.client.clone(), namespace);

		hpa_api
			.patch(
				&format!("hpa-{}", spec.deployment.id),
				&PatchParams::apply(&format!("hpa-{}", spec.deployment.id)),
				&Patch::Apply(kubernetes_hpa),
			)
			.await?;
	} else {
		let kubernetes_sts = StatefulSet {
			metadata,
			spec: Some(StatefulSetSpec {
				replicas,
				selector,
				service_name: format!("service-{}", spec.deployment.id),
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

		// Create the stateful set defined above
		trace!("creating stateful set");
		let sts_api = Api::<StatefulSet>::namespaced(ctx.client.clone(), namespace);

		sts_api
			.patch(
				&format!("sts-{}", spec.deployment.id),
				&PatchParams::apply(&format!("sts-{}", spec.deployment.id)),
				&Patch::Apply(kubernetes_sts),
			)
			.await?;

		// This is because if a user wanted to add the volume to there
		// deployment then a deployment will be converted to sts
		trace!("deleting the deployment set if there are any");

		Api::<KubeDeployment>::namespaced(ctx.client.clone(), namespace)
			.delete_opt(
				&format!("deployment-{}", spec.deployment.id),
				&DeleteParams::default(),
			)
			.await?;

		// Delete the HPA, if any
		trace!("deleting horizontal pod autoscaler");
		let hpa_api = Api::<HorizontalPodAutoscaler>::namespaced(ctx.client.clone(), namespace);

		hpa_api
			.delete_opt(
				&format!("hpa-{}", spec.deployment.id),
				&DeleteParams::default(),
			)
			.await?;
	}

	// For a deployment has more than one replica, then only we can use
	// pod-disruption-budget to move pods between nodes without any down time.
	// Even with hpa of max=4 but min=1 if the number of pods currently running
	// is 1, then it will block the node drain
	if spec.running_details.min_horizontal_scale > 1 {
		// Create pdb for deployment alone
		// For sts, we can't use pdb as it involves state handling
		// see: https://kubernetes.io/docs/tasks/run-application/configure-pdb/#think-about-how-your-application-reacts-to-disruptions
		trace!("creating pod disruption budget");

		let pdb = PodDisruptionBudget {
			metadata: ObjectMeta {
				name: Some(format!("pdb-{}", spec.deployment.id)),
				namespace: Some(namespace.to_string()),
				labels: Some(labels.clone()),
				owner_references: Some(vec![owner_reference.clone()]),
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

		Api::<PodDisruptionBudget>::namespaced(ctx.client.clone(), namespace)
			.patch(
				&format!("pdb-{}", spec.deployment.id),
				&PatchParams::apply(&format!("pdb-{}", spec.deployment.id)),
				&Patch::Apply(pdb),
			)
			.await?;
	} else {
		trace!("min replica is not more than one, so deleting the pdb (if present)");

		Api::<PodDisruptionBudget>::namespaced(ctx.client.clone(), namespace)
			.delete_opt(
				&format!("pdb-{}", spec.deployment.id),
				&DeleteParams::default(),
			)
			.await?;
	}

	// Create the ingress defined above
	trace!("creating ingress");
	Api::<Ingress>::namespaced(ctx.client.clone(), namespace)
		.patch(
			&format!("ingress-{}", spec.deployment.id),
			&PatchParams::apply(&format!("ingress-{}", spec.deployment.id)),
			&Patch::Apply(Ingress {
				metadata: ObjectMeta {
					name: Some(format!("ingress-{}", spec.deployment.id)),
					annotations: Some(
						[(
							"kubernetes.io/ingress.class".to_string(),
							"nginx".to_string(),
						)]
						.into(),
					),
					owner_references: Some(vec![owner_reference.clone()]),
					..ObjectMeta::default()
				},
				spec: Some(IngressSpec {
					rules: Some(
						spec.running_details
							.ports
							.iter()
							.filter(|(_, port_type)| *port_type == &ExposedPortType::Http)
							.map(|(port, _)| IngressRule {
								host: Some(format!(
									"{}-{}.{}.onpatr.cloud",
									port, spec.deployment.id, ctx.region_id,
								)),
								http: Some(HTTPIngressRuleValue {
									paths: vec![HTTPIngressPath {
										backend: IngressBackend {
											service: Some(IngressServiceBackend {
												name: format!("service-{}", spec.deployment.id),
												port: Some(ServiceBackendPort {
													number: Some(port.value().into()),
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
							.collect(),
					),
					..IngressSpec::default()
				}),
				..Ingress::default()
			}),
		)
		.await?;

	Ok(Action::requeue(Duration::from_secs(3600)))
}
