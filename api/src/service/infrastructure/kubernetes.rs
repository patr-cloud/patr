use std::{collections::BTreeMap, ops::DerefMut};

use api_models::{
	models::workspace::infrastructure::{
		deployment::{
			Deployment,
			DeploymentRunningDetails,
			DeploymentStatus,
			EnvironmentVariableValue,
			ExposedPortType,
		},
		static_site::{StaticSite, StaticSiteDetails},
	},
	utils::Uuid,
};
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
	config::{
		AuthInfo,
		Cluster,
		Context,
		Kubeconfig,
		NamedAuthInfo,
		NamedCluster,
		NamedContext,
	},
	core::{ApiResource, DynamicObject, ObjectMeta, TypeMeta},
	Api,
	Config,
	Error as KubeError,
};
use serde_json::json;

use crate::{
	db,
	error,
	service::{self, infrastructure::digitalocean},
	utils::{
		constants::{request_keys, ResourceOwnerType},
		settings::Settings,
		Error,
	},
	Database,
};

pub(super) async fn update_kubernetes_static_site(
	workspace_id: &Uuid,
	static_site: &StaticSite,
	_static_site_details: &StaticSiteDetails,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client = get_kubernetes_config(config).await?;
	// new name for the docker image

	let namespace = workspace_id.as_str();
	log::trace!(
		"request_id: {} - generating deployment configuration",
		request_id
	);

	let mut selector = BTreeMap::new();
	selector.insert("app".to_string(), "static-sites-proxy".to_string());

	let kubernetes_service = Service {
		metadata: ObjectMeta {
			name: Some(format!("service-{}", static_site.id)),
			..ObjectMeta::default()
		},
		spec: Some(ServiceSpec {
			type_: Some("ClusterIP".to_string()),
			selector: Some(selector),
			ports: Some(vec![ServicePort {
				port: 80,
				name: Some("http".to_string()),
				target_port: Some(IntOrString::Int(80)),
				..ServicePort::default()
			}]),
			..ServiceSpec::default()
		}),
		..Service::default()
	};
	// Create the service defined above
	log::trace!("request_id: {} - creating ClusterIP service", request_id);
	let service_api: Api<Service> =
		Api::namespaced(kubernetes_client.clone(), namespace);
	service_api
		.patch(
			&format!("service-{}", static_site.id),
			&PatchParams::apply(&format!("service-{}", static_site.id)),
			&Patch::Apply(kubernetes_service),
		)
		.await?
		.status
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;
	log::trace!("request_id: {} - created ExternalName service", request_id);
	let mut annotations: BTreeMap<String, String> = BTreeMap::new();
	annotations.insert(
		"kubernetes.io/ingress.class".to_string(),
		"nginx".to_string(),
	);
	annotations.insert(
		"nginx.ingress.kubernetes.io/upstream-vhost".to_string(),
		format!("{}.patr.cloud", static_site.id),
	);

	annotations.insert(
		"cert-manager.io/issuer".to_string(),
		config.kubernetes.cert_issuer.clone(),
	);
	let ingress_rule = vec![IngressRule {
		host: Some(format!("{}.patr.cloud", static_site.id)),
		http: Some(HTTPIngressRuleValue {
			paths: vec![HTTPIngressPath {
				backend: IngressBackend {
					service: Some(IngressServiceBackend {
						name: format!("service-{}", static_site.id),
						port: Some(ServiceBackendPort {
							number: Some(80),
							..ServiceBackendPort::default()
						}),
					}),
					..IngressBackend::default()
				},
				path: Some("/".to_string()),
				path_type: Some("Prefix".to_string()),
			}],
		}),
	}];

	log::trace!(
		"request_id: {} - adding patr domain config to ingress",
		request_id
	);
	let patr_domain_tls = vec![IngressTLS {
		hosts: Some(vec![format!("{}.patr.cloud", static_site.id)]),
		secret_name: Some("tls-domain-wildcard-patr-cloud".to_string()),
	}];
	log::trace!(
		"request_id: {} - creating https certificates for domain",
		request_id
	);
	let kubernetes_ingress = Ingress {
		metadata: ObjectMeta {
			name: Some(format!("ingress-{}", static_site.id)),
			annotations: Some(annotations),
			..ObjectMeta::default()
		},
		spec: Some(IngressSpec {
			rules: Some(ingress_rule),
			tls: Some(patr_domain_tls),
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
			&format!("ingress-{}", static_site.id),
			&PatchParams::apply(&format!("ingress-{}", static_site.id)),
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
		static_site.id
	);
	Ok(())
}

pub(super) async fn delete_static_site_from_k8s(
	workspace_id: &Uuid,
	static_site_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client = get_kubernetes_config(config).await?;

	let namespace = workspace_id.as_str();
	log::trace!(
		"request_id: {} - deleting service: service-{}",
		request_id,
		static_site_id
	);

	if !service_exists(static_site_id, kubernetes_client.clone(), namespace)
		.await?
	{
		log::trace!(
			"request_id: {} - App doesn't exist as {}",
			request_id,
			static_site_id
		);
		log::trace!(
			"request_id: {} - deployment deleted successfully!",
			request_id
		);
		Ok(())
	} else {
		log::trace!(
			"request_id: {} - site exists as {}",
			request_id,
			static_site_id
		);

		Api::<Service>::namespaced(kubernetes_client.clone(), namespace)
			.delete(
				&format!("service-{}", static_site_id),
				&DeleteParams::default(),
			)
			.await?;
		Api::<Ingress>::namespaced(kubernetes_client, namespace)
			.delete(
				&format!("ingress-{}", static_site_id),
				&DeleteParams::default(),
			)
			.await?;
		log::trace!(
			"request_id: {} - deployment deleted successfully!",
			request_id
		);
		Ok(())
	}
}

pub async fn update_kubernetes_deployment(
	workspace_id: &Uuid,
	deployment: &Deployment,
	full_image: &str,
	running_details: &DeploymentRunningDetails,
	config: &Settings,
) -> Result<(), Error> {
	let kubernetes_client = get_kubernetes_config(config).await?;

	let request_id = Uuid::new_v4();
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

	// TODO get this from machine type
	let machine_type = [
		("memory".to_string(), Quantity("1G".to_string())),
		("cpu".to_string(), Quantity("1.0".to_string())),
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
									container_port: *port as i32,
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
						port: *port as i32,
						target_port: Some(IntOrString::Int(*port as i32)),
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

	let mut annotations: BTreeMap<String, String> = BTreeMap::new();
	annotations.insert(
		"kubernetes.io/ingress.class".to_string(),
		"nginx".to_string(),
	);

	log::trace!(
		"request_id: {} - creating https certificates for domain",
		request_id
	);

	annotations.insert(
		"cert-manager.io/issuer".to_string(),
		config.kubernetes.cert_issuer.clone(),
	);

	// Get all domain names for domain IDs
	let mut entry_points = Vec::with_capacity(running_details.urls.len());
	for url in &running_details.urls {
		let domain = db::get_workspace_domain_by_id(
			service::get_app().database.acquire().await?.deref_mut(),
			&url.domain_id,
		)
		.await?
		.status(500)?;
		entry_points.push((
			url.sub_domain.clone(),
			domain,
			url.path.clone(),
			url.port,
		));
	}

	let domain_ingress_rules = entry_points
		.iter()
		.map(|(sub_domain, domain, path, port)| IngressRule {
			host: Some(format!("{}.{}", sub_domain, domain.name)),
			http: Some(HTTPIngressRuleValue {
				paths: vec![HTTPIngressPath {
					backend: IngressBackend {
						service: Some(IngressServiceBackend {
							name: format!("service-{}", deployment.id),
							port: Some(ServiceBackendPort {
								number: Some(*port as i32),
								..ServiceBackendPort::default()
							}),
						}),
						..IngressBackend::default()
					},
					path: Some(path.clone()),
					path_type: Some("Prefix".to_string()),
				}],
			}),
		})
		.chain(
			running_details
				.ports
				.iter()
				.filter(|(_, port_type)| *port_type == &ExposedPortType::Http)
				.map(|(port, _)| IngressRule {
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
										number: Some(*port as i32),
										name: Some(format!("port-{}", port)),
									}),
								}),
								..IngressBackend::default()
							},
							..HTTPIngressPath::default()
						}],
					}),
				}),
		)
		.collect::<Vec<_>>();

	let mut domain_tls =
		Vec::with_capacity(entry_points.len() + running_details.ports.len());
	for (port, port_type) in &running_details.ports {
		if port_type != &ExposedPortType::Http {
			continue;
		}
		domain_tls.push(IngressTLS {
			hosts: Some(vec![format!("{}-{}.patr.cloud", port, deployment.id)]),
			// TODO rename patr-domain to {patr-domain.id} below
			secret_name: Some("tls-domain-wildcard-patr-domain".to_string()),
		});
	}
	for (sub_domain, domain, ..) in &entry_points {
		domain_tls.push(IngressTLS {
			hosts: Some(vec![format!("{}.{}", sub_domain, domain.name)]),
			secret_name: Some(
				// Change this to check if the domain is patr-controlled or
				// user controlled
				if domain.domain_type == ResourceOwnerType::Business {
					format!("tls-domain-{}-{}", sub_domain, domain.id)
				} else {
					format!("tls-domain-wildcard-{}", domain.id)
				},
			),
		});
	}

	let kubernetes_ingress = Ingress {
		metadata: ObjectMeta {
			name: Some(format!("ingress-{}", deployment.id)),
			annotations: Some(annotations),
			..ObjectMeta::default()
		},
		spec: Some(IngressSpec {
			rules: Some(domain_ingress_rules),
			tls: Some(domain_tls),
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

pub(super) async fn delete_kubernetes_deployment(
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - deleting the image from registry",
		request_id
	);
	let kubernetes_client = get_kubernetes_config(config).await?;

	if !deployment_exists(
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

pub(super) async fn get_container_logs(
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	request_id: Uuid,
	config: &Settings,
) -> Result<String, Error> {
	// TODO: interact with prometheus to get the logs

	let kubernetes_client = get_kubernetes_config(config).await?;

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

// TODO: add logs
pub async fn create_certificates(
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	static_site_id: &Uuid,
	domain_list: Vec<String>,
	config: &Settings,
) -> Result<(), Error> {
	let kubernetes_client = get_kubernetes_config(config).await?;

	let certificate_resource = ApiResource {
		group: "cert-manager.io".to_string(),
		version: "v1".to_string(),
		api_version: "cert-manager.io/v1".to_string(),
		kind: "certificate".to_string(),
		plural: "certificates".to_string(),
	};

	// TODO: use yaml raw string to be converted in to value
	let certificate_data = json!(
		{
			"spec": {
				// TODO: change this name
				"secretName": format!("tls-domain-{}", deployment_id),
				"dnsNames": domain_list,
				"issuerRef": {
					"name": config.kubernetes.cert_issuer,
					// TODO: change this to cluster-issuer
					"kind": "Issuer",
					"group": "cert-manager.io"
				},
			}
		}
	);

	let certificate = DynamicObject {
		types: Some(TypeMeta {
			api_version: "cert-manager.io/v1".to_string(),
			kind: "certificate".to_string(),
		}),
		metadata: ObjectMeta {
			annotations: None,
			cluster_name: None,
			creation_timestamp: None,
			deletion_grace_period_seconds: None,
			deletion_timestamp: None,
			finalizers: None,
			generate_name: None,
			generation: None,
			labels: None,
			managed_fields: None,
			name: Some(format!("cert-domain-{}", deployment_id)),
			namespace: None,
			owner_references: None,
			resource_version: None,
			self_link: None,
			uid: None,
		},
		data: certificate_data,
	};

	let certificate_api = Api::<DynamicObject>::namespaced(
		kubernetes_client,
		workspace_id,
		certificate_resource,
	)
	.create(&PostParams::default(), &certificate)
	.await?;

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

	let kubernetes_client = get_kubernetes_config(config).await?;
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

async fn service_exists(
	static_site_id: &Uuid,
	kubernetes_client: kube::Client,
	namespace: &str,
) -> Result<bool, KubeError> {
	let deployment_app =
		Api::<Service>::namespaced(kubernetes_client, namespace)
			.get(&format!("service-{}", static_site_id))
			.await;
	if let Err(KubeError::Api(error)) = deployment_app {
		if error.code == 404 {
			return Ok(false);
		} else {
			return Err(KubeError::Api(error));
		}
	}

	Ok(true)
}

async fn get_kubernetes_config(
	config: &Settings,
) -> Result<kube::Client, Error> {
	let config = Config::from_custom_kubeconfig(
		Kubeconfig {
			preferences: None,
			clusters: vec![NamedCluster {
				name: config.kubernetes.cluster_name.clone(),
				cluster: Cluster {
					server: config.kubernetes.cluster_url.clone(),
					insecure_skip_tls_verify: None,
					certificate_authority: None,
					certificate_authority_data: Some(
						config.kubernetes.certificate_authority_data.clone(),
					),
					proxy_url: None,
					extensions: None,
				},
			}],
			auth_infos: vec![NamedAuthInfo {
				name: config.kubernetes.auth_name.clone(),
				auth_info: AuthInfo {
					username: Some(config.kubernetes.auth_username.clone()),
					token: Some(config.kubernetes.auth_token.clone()),
					..Default::default()
				},
			}],
			contexts: vec![NamedContext {
				name: config.kubernetes.context_name.clone(),
				context: Context {
					cluster: config.kubernetes.cluster_name.clone(),
					user: config.kubernetes.auth_username.clone(),
					extensions: None,
					namespace: None,
				},
			}],
			current_context: Some(config.kubernetes.context_name.clone()),
			extensions: None,
			kind: Some("Config".to_string()),
			api_version: Some("v1".to_string()),
		},
		&Default::default(),
	)
	.await?;

	let client = kube::Client::try_from(config)?;

	Ok(client)
}

async fn deployment_exists(
	deployment_id: &Uuid,
	kubernetes_client: kube::Client,
	namespace: &str,
) -> Result<bool, KubeError> {
	let deployment_app =
		Api::<K8sDeployment>::namespaced(kubernetes_client, namespace)
			.get(&format!("deployment-{}", deployment_id))
			.await;

	if let Err(KubeError::Api(error)) = deployment_app {
		if error.code == 404 {
			return Ok(false);
		} else {
			return Err(KubeError::Api(error));
		}
	}

	Ok(true)
}
