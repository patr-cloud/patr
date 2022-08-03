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
		apps::v1::{Deployment as K8sDeployment, DeploymentSpec},
		autoscaling::v1::{
			CrossVersionObjectReference,
			HorizontalPodAutoscaler,
			HorizontalPodAutoscalerSpec,
		},
		core::v1::{
			Container,
			ContainerPort,
			EnvVar,
			HTTPGetAction,
			LocalObjectReference,
			Namespace,
			PodSpec,
			PodTemplateSpec,
			Probe,
			ResourceRequirements,
			Service,
			ServicePort,
			ServiceSpec,
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
};
use kube::{
	api::{DeleteParams, Patch, PatchParams, PostParams},
	core::{ErrorResponse, ObjectMeta},
	Api,
	Error as KubeError,
};

use crate::{
	db,
	error,
	models::deployment::{self},
	utils::{constants::request_keys, settings::Settings, Error},
	Database,
};

pub async fn update_kubernetes_deployment(
	workspace_id: &Uuid,
	deployment: &Deployment,
	full_image: &str,
	digest: Option<&str>,
	running_details: &DeploymentRunningDetails,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let cluster_config =
		super::get_kubernetes_cluster_config(config, &deployment.region)
			.unwrap_or_else(|| {
				super::get_kubernetes_default_cluster_config(config)
			});
	let kubernetes_client =
		super::get_kubernetes_client(cluster_config).await?;

	// the namespace is workspace id
	let namespace = workspace_id.as_str();

	// check whether namespace exists in the given cluster
	if !super::namespace_exists(namespace, kubernetes_client.clone()).await? {
		log::trace!("request_id: {} - creating namespace", request_id);
		let kubernetes_namespace = Namespace {
			metadata: ObjectMeta {
				name: Some(namespace.to_string()),
				..ObjectMeta::default()
			},
			..Namespace::default()
		};
		let namespace_api = Api::<Namespace>::all(kubernetes_client.clone());
		namespace_api
			.create(&PostParams::default(), &kubernetes_namespace)
			.await?;
		log::trace!("request_id: {} - namespace created", request_id);
	}

	if !super::secret_exists(
		"tls-domain-wildcard-patr-cloud",
		kubernetes_client.clone(),
		namespace,
	)
	.await?
	{
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	}

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
								.iter()
								.map(|(port, _)| ContainerPort {
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
									port: IntOrString::Int(probe.port as i32),
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
								])
								.collect::<Vec<_>>(),
						),
						resources: Some(ResourceRequirements {
							limits: Some(machine_type.clone()),
							..ResourceRequirements::default()
						}),
						..Container::default()
					}],
					image_pull_secrets: deployment
						.registry
						.is_patr_registry()
						.then(|| {
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
			..DeploymentSpec::default()
		}),
		..K8sDeployment::default()
	};

	// Create the deployment defined above
	log::trace!("request_id: {} - creating deployment", request_id);
	let deployment_api =
		Api::<K8sDeployment>::namespaced(kubernetes_client.clone(), namespace);

	deployment_api
		.patch(
			&format!("deployment-{}", deployment.id),
			&PatchParams::apply(&format!("deployment-{}", deployment.id)),
			&Patch::Apply(kubernetes_deployment),
		)
		.await?;

	let kubernetes_service = Service {
		metadata: ObjectMeta {
			name: Some(format!("service-{}", deployment.id)),
			..ObjectMeta::default()
		},
		spec: Some(ServiceSpec {
			ports: Some(
				running_details
					.ports
					.iter()
					.map(|(port, _)| ServicePort {
						port: port.value() as i32,
						target_port: Some(
							IntOrString::Int(port.value() as i32),
						),
						name: Some(format!("port-{}", port)),
						..ServicePort::default()
					})
					.collect::<Vec<_>>(),
			),
			selector: Some(labels),
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

	let annotations = [
		(
			"kubernetes.io/ingress.class".to_string(),
			"nginx".to_string(),
		),
		(
			"cert-manager.io/cluster-issuer".to_string(),
			cluster_config.cert_issuer_dns.clone(),
		),
	]
	.into_iter()
	.collect();

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
			annotations: Some(annotations),
			..ObjectMeta::default()
		},
		spec: Some(IngressSpec {
			rules: Some(default_ingress_rules),
			tls: Some(default_tls_rules),
			..IngressSpec::default()
		}),
		..Ingress::default()
	};

	// Create the ingress defined above
	log::trace!("request_id: {} - creating ingress", request_id);
	let ingress_api = Api::<Ingress>::namespaced(kubernetes_client, namespace);

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
			.map(|(port, _)| format!("{}-{}.patr.cloud", port, deployment.id))
			.collect::<Vec<_>>()
			.join(",")
	);

	Ok(())
}

pub async fn delete_kubernetes_deployment(
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let cluster_config = super::get_kubernetes_default_cluster_config(config);
	let kubernetes_client =
		super::get_kubernetes_client(cluster_config).await?;

	if super::deployment_exists(
		deployment_id,
		kubernetes_client.clone(),
		workspace_id.as_str(),
	)
	.await?
	{
		log::trace!(
			"request_id: {} - Deployment exists as deployment-{}",
			request_id,
			deployment_id
		);

		log::trace!("request_id: {} - deleting the deployment", request_id);

		Api::<K8sDeployment>::namespaced(
			kubernetes_client.clone(),
			workspace_id.as_str(),
		)
		.delete(
			&format!("deployment-{}", deployment_id),
			&DeleteParams::default(),
		)
		.await?;
	} else {
		log::trace!(
			"request_id: {} - No deployment found with name deployment-{} in namespace: {}",
			request_id,
			deployment_id,
			workspace_id,
		);
	}

	if super::service_exists(
		deployment_id,
		kubernetes_client.clone(),
		workspace_id.as_str(),
	)
	.await?
	{
		log::trace!(
			"request_id: {} - Service exists as service-{}",
			request_id,
			deployment_id
		);
		log::trace!("request_id: {} - deleting the service", request_id);
		Api::<Service>::namespaced(
			kubernetes_client.clone(),
			workspace_id.as_str(),
		)
		.delete(
			&format!("service-{}", deployment_id),
			&DeleteParams::default(),
		)
		.await?;
	} else {
		log::trace!(
			"request_id: {} - No deployment service found with name deployment-{} in namespace: {}",
			request_id,
			deployment_id,
			workspace_id,
		);
	}

	if super::hpa_exists(
		deployment_id,
		kubernetes_client.clone(),
		workspace_id.as_str(),
	)
	.await?
	{
		log::trace!(
			"request_id: {} - hpa exists as hpa-{}",
			request_id,
			deployment_id
		);

		log::trace!("request_id: {} - deleting the hpa", request_id);

		Api::<HorizontalPodAutoscaler>::namespaced(
			kubernetes_client.clone(),
			workspace_id.as_str(),
		)
		.delete(&format!("hpa-{}", deployment_id), &DeleteParams::default())
		.await?;
	} else {
		log::trace!(
			"request_id: {} - No hpa found with name hpa-{} in namespace: {}",
			request_id,
			deployment_id,
			workspace_id,
		);
	}

	if super::ingress_exists(
		deployment_id,
		kubernetes_client.clone(),
		workspace_id.as_str(),
	)
	.await?
	{
		log::trace!(
			"request_id: {} - Ingress exists as ingress-{}",
			request_id,
			deployment_id
		);
		log::trace!("request_id: {} - deleting the ingress", request_id);
		Api::<Ingress>::namespaced(kubernetes_client, workspace_id.as_str())
			.delete(
				&format!("ingress-{}", deployment_id),
				&DeleteParams::default(),
			)
			.await?;
	} else {
		log::trace!(
			"request_id: {} - No deployment ingress found with name deployment-{} in namespace: {}",
			request_id,
			deployment_id,
			workspace_id,
		);
	}

	log::trace!(
		"request_id: {} - deployment deleted successfully!",
		request_id
	);
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

	let cluster_config = super::get_kubernetes_default_cluster_config(config);
	let kubernetes_client =
		super::get_kubernetes_client(cluster_config).await?;
	let deployment_result =
		Api::<K8sDeployment>::namespaced(kubernetes_client.clone(), namespace)
			.get(&format!("deployment-{}", deployment.id))
			.await;
	let deployment_status = match deployment_result {
		Err(KubeError::Api(ErrorResponse { code: 404, .. })) => {
			// TODO: This is a temporary fix to solve issue #361.
			// Need to find a better solution to do this
			return Ok(DeploymentStatus::Deploying);
		}
		Err(err) => return Err(err.into()),
		Ok(deployment) => deployment
			.status
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?,
	};

	if deployment_status.available_replicas ==
		Some(deployment.min_horizontal_scale.into())
	{
		Ok(DeploymentStatus::Running)
	} else if deployment_status.available_replicas <=
		Some(deployment.min_horizontal_scale.into())
	{
		Ok(DeploymentStatus::Deploying)
	} else {
		Ok(DeploymentStatus::Errored)
	}
}
