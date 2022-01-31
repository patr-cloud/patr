use std::collections::BTreeMap;

use api_models::{
	models::workspace::infrastructure::deployment::{
		Deployment,
		DeploymentRegistry,
		DeploymentRunningDetails,
		DeploymentStatus,
		EnvironmentVariableValue,
		ExposedPortType,
	},
	utils::Uuid,
};
use chrono::{DateTime, Local};
use eve_rs::AsError;
use k8s_openapi::{
	api::{
		apps::v1::{Deployment as K8sDeployment, DeploymentSpec},
		core::v1::{
			Container,
			ContainerPort,
			EnvVar,
			LocalObjectReference,
			Pod,
			PodSpec,
			PodTemplateSpec,
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
	api::{DeleteParams, ListParams, LogParams, Patch, PatchParams},
	core::ObjectMeta,
	Api,
};
use serde_json::json;

use crate::{
	db,
	error,
	models::deployment,
	service::infrastructure::digitalocean,
	utils::{constants::request_keys, settings::Settings, Error},
	Database,
};

pub async fn update_kubernetes_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	deployment: &Deployment,
	full_image: &str,
	running_details: &DeploymentRunningDetails,
	config: &Settings,
	request_id: &Uuid,
	user_id: Option<&Uuid>,
	login_id: Option<&Uuid>,
	patr_action: bool,
	time_now: DateTime<Local>,
) -> Result<(), Error> {
	let workspace_audit_log_id =
		db::generate_new_workspace_audit_log_id(connection).await?;

	let edit_permission_id = db::get_all_permissions(connection)
		.await?
		.into_iter()
		.find_map(|permission| {
			if permission.name == "workspace::infrastructure::deployment::edit"
			{
				Some(permission)
			} else {
				None
			}
		})
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?
		.id;

	let registry_name = match deployment.registry.clone() {
		DeploymentRegistry::PatrRegistry { .. } => {
			"registry.patr.cloud".to_string()
		}
		DeploymentRegistry::ExternalRegistry { registry, .. } => registry,
	};

	let metadata = json!(
		{

			"id": &deployment.id,
			"name": &deployment.name,
			"registry": &registry_name,
			"image": full_image,
			"deploymentStatus": DeploymentStatus::Running.to_string(),
			"machineType": deployment.machine_type,
			"deployOnPush": running_details.deploy_on_push, // true or false
			"horizontalScale": running_details.min_horizontal_scale,
			"region": deployment.region
		}
	);

	db::create_workspace_audit_log(
		connection,
		workspace_id,
		&workspace_audit_log_id,
		"0.0.0.0",
		time_now,
		user_id,
		login_id,
		&deployment.id,
		&edit_permission_id,
		request_id,
		&metadata,
		patr_action,
		true,
	)
	.await?;

	let kubernetes_client = super::get_kubernetes_config(config).await?;

	log::trace!(
		"Deploying the container with id: {} on kubernetes with request_id: {}",
		deployment.id,
		request_id,
	);

	// new name for the docker image
	let image_name = if deployment.registry.is_patr_registry() {
		format!(
			"registry.digitalocean.com/{}/{}",
			config.digitalocean.registry, deployment.id,
		)
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

	// the namespace is workspace id
	let namespace = workspace_id.as_str();

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
	]
	.into_iter()
	.collect::<BTreeMap<_, _>>();

	log::trace!(
		"request_id: {} - generating deployment configuration",
		request_id
	);

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
						env: Some(
							running_details
								.environment_variables
								.iter()
								.filter_map(|(name, value)| {
									use EnvironmentVariableValue::*;
									Some(EnvVar {
										name: name.to_string(),
										value: Some(match value {
											String(value) => value.to_string(),
											Secret { .. } => {
												return None;
											}
										}),
										..EnvVar::default()
									})
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
							requests: Some(machine_type),
						}),
						..Container::default()
					}],
					image_pull_secrets: Some(vec![LocalObjectReference {
						name: Some("regcred".to_string()),
					}]),
					..PodSpec::default()
				}),
				metadata: Some(ObjectMeta {
					labels: Some(labels.clone()),
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
		.await?
		.status
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

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
	let service_api: Api<Service> =
		Api::namespaced(kubernetes_client.clone(), namespace);

	service_api
		.patch(
			&format!("service-{}", deployment.id),
			&PatchParams::apply(&format!("service-{}", deployment.id)),
			&Patch::Apply(kubernetes_service),
		)
		.await?
		.status
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let annotations = [
		(
			"kubernetes.io/ingress.class".to_string(),
			"nginx".to_string(),
		),
		(
			"cert-manager.io/issuer".to_string(),
			config.kubernetes.cert_issuer.clone(),
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
					hosts: Some(vec![format!(
						"{}-{}.patr.cloud",
						port, deployment.id
					)]),
					secret_name: Some(
						"tls-domain-wildcard-patr-cloud".to_string(),
					),
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
	let ingress_api: Api<Ingress> =
		Api::namespaced(kubernetes_client, namespace);

	ingress_api
		.patch(
			&format!("ingress-{}", deployment.id),
			&PatchParams::apply(&format!("ingress-{}", deployment.id)),
			&Patch::Apply(kubernetes_ingress),
		)
		.await?
		.status
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	log::trace!("request_id: {} - deployment created", request_id);

	log::trace!(
		"request_id: {} - App ingress is at {}.patr.cloud",
		request_id,
		deployment.id
	);

	Ok(())
}

pub async fn delete_kubernetes_deployment(
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - deleting the image from registry",
		request_id
	);
	let kubernetes_client = super::get_kubernetes_config(config).await?;

	if !super::deployment_exists(
		deployment_id,
		kubernetes_client.clone(),
		workspace_id.as_str(),
	)
	.await?
	{
		log::trace!(
			"request_id: {} - App doesn't exist as {}",
			request_id,
			deployment_id
		);
		log::trace!(
			"request_id: {} - deployment deleted successfully!",
			request_id
		);
		return Ok(());
	}

	log::trace!(
		"request_id: {} - App exists as {}",
		request_id,
		deployment_id
	);
	digitalocean::delete_image_from_digitalocean_registry(
		deployment_id,
		config,
		request_id,
	)
	.await?;

	log::trace!("request_id: {} - deleting the deployment", request_id);

	Api::<K8sDeployment>::namespaced(
		kubernetes_client.clone(),
		workspace_id.as_str(),
	)
	.delete(deployment_id.as_str(), &DeleteParams::default())
	.await?;
	Api::<Service>::namespaced(
		kubernetes_client.clone(),
		workspace_id.as_str(),
	)
	.delete(
		&format!("service-{}", deployment_id),
		&DeleteParams::default(),
	)
	.await?;
	Api::<Ingress>::namespaced(kubernetes_client, workspace_id.as_str())
		.delete(
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

pub async fn get_container_logs(
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<String, Error> {
	// TODO: interact with prometheus to get the logs

	let kubernetes_client = super::get_kubernetes_config(config).await?;

	log::trace!(
		"request_id: {} - retreiving deployment info from db",
		request_id
	);

	// TODO: change log to stream_log when eve gets websockets
	// TODO: change customise LogParams for different types of logs
	// TODO: this is a temporary log retrieval method, use prometheus to get the
	// logs
	let pod_api =
		Api::<Pod>::namespaced(kubernetes_client, workspace_id.as_str());

	let pod_name = pod_api
		.list(&ListParams {
			label_selector: Some(format!(
				"{}={}",
				request_keys::DEPLOYMENT_ID,
				deployment_id
			)),
			..ListParams::default()
		})
		.await?
		.items
		.into_iter()
		.next()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?
		.metadata
		.name
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let deployment_logs =
		pod_api.logs(&pod_name, &LogParams::default()).await?;

	log::trace!("request_id: {} - logs retreived successfully!", request_id);
	Ok(deployment_logs)
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

	let kubernetes_client = super::get_kubernetes_config(config).await?;
	let deployment_status =
		Api::<K8sDeployment>::namespaced(kubernetes_client.clone(), namespace)
			.get(deployment.id.as_str())
			.await?
			.status
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;

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
