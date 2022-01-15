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
		managed_urls::{ManagedUrl, ManagedUrlType},
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
	core::ObjectMeta,
	Api,
	Config,
	Error as KubeError,
};

use crate::{
	db,
	error,
	models::deployment,
	service::{self, infrastructure::digitalocean},
	utils::{constants::request_keys, settings::Settings, Error},
	Database,
};

pub async fn update_kubernetes_static_site(
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
					..Default::default()
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

pub async fn delete_kubernetes_static_site(
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

	if service_exists(static_site_id, kubernetes_client.clone(), namespace)
		.await?
	{
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
	} else {
		log::trace!(
			"request_id: {} - App doesn't exist as {}",
			request_id,
			static_site_id
		);
	}

	log::trace!(
		"request_id: {} - static site deleted successfully!",
		request_id
	);
	Ok(())
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
										number: Some(*port as i32),
										name: Some(format!("port-{}", port)),
									}),
								}),
								..Default::default()
							},
							..HTTPIngressPath::default()
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

pub async fn update_kubernetes_managed_url(
	workspace_id: &Uuid,
	managed_url: &ManagedUrl,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client = get_kubernetes_config(config).await?;

	let namespace = workspace_id.as_str();
	log::trace!(
		"request_id: {} - generating deployment configuration",
		request_id
	);
	let domain = db::get_workspace_domain_by_id(
		service::get_app().database.acquire().await?.deref_mut(),
		&managed_url.domain_id,
	)
	.await?
	.status(500)?;

	let (ingress, annotations) = match &managed_url.url_type {
		ManagedUrlType::ProxyDeployment {
			deployment_id,
			port,
		} => (
			IngressRule {
				host: Some(format!(
					"{}.{}",
					managed_url.sub_domain, domain.name
				)),
				http: Some(HTTPIngressRuleValue {
					paths: vec![HTTPIngressPath {
						backend: IngressBackend {
							service: Some(IngressServiceBackend {
								name: format!("service-{}", deployment_id),
								port: Some(ServiceBackendPort {
									number: Some(*port as i32),
									..ServiceBackendPort::default()
								}),
							}),
							..Default::default()
						},
						path: Some(managed_url.path.to_string()),
						path_type: Some("Prefix".to_string()),
					}],
				}),
			},
			[
				(
					"kubernetes.io/ingress.class".to_string(),
					"nginx".to_string(),
				),
				(
					"nginx.ingress.kubernetes.io/upstream-vhost".to_string(),
					format!("{}.{}", managed_url.sub_domain, domain.name),
				),
				(
					"cert-manager.io/issuer".to_string(),
					config.kubernetes.cert_issuer.clone(),
				),
			]
			.into_iter()
			.collect(),
		),
		ManagedUrlType::ProxyStaticSite { static_site_id } => (
			IngressRule {
				host: Some(format!(
					"{}.{}",
					managed_url.sub_domain, domain.name
				)),
				http: Some(HTTPIngressRuleValue {
					paths: vec![HTTPIngressPath {
						backend: IngressBackend {
							service: Some(IngressServiceBackend {
								name: format!("service-{}", static_site_id),
								port: Some(ServiceBackendPort {
									number: Some(80),
									..ServiceBackendPort::default()
								}),
							}),
							..Default::default()
						},
						path: Some(managed_url.path.to_string()),
						path_type: Some("Prefix".to_string()),
					}],
				}),
			},
			[
				(
					"kubernetes.io/ingress.class".to_string(),
					"nginx".to_string(),
				),
				(
					"nginx.ingress.kubernetes.io/upstream-vhost".to_string(),
					format!("{}.{}", managed_url.sub_domain, domain.name),
				),
				(
					"cert-manager.io/issuer".to_string(),
					config.kubernetes.cert_issuer.clone(),
				),
			]
			.into_iter()
			.collect(),
		),
		ManagedUrlType::ProxyUrl { url } => {
			let kubernetes_service = Service {
				metadata: ObjectMeta {
					name: Some(format!("service-{}", managed_url.id)),
					..ObjectMeta::default()
				},
				spec: Some(ServiceSpec {
					type_: Some("ExternalName".to_string()),
					external_name: Some(url.clone()),
					ports: Some(vec![ServicePort {
						name: Some("https".to_string()),
						port: 443,
						protocol: Some("TCP".to_string()),
						target_port: Some(IntOrString::Int(443)),
						..ServicePort::default()
					}]),
					..ServiceSpec::default()
				}),
				..Service::default()
			};
			// Create the service defined above
			log::trace!(
				"request_id: {} - creating ExternalName service",
				request_id
			);
			let service_api: Api<Service> =
				Api::namespaced(kubernetes_client.clone(), namespace);
			service_api
				.patch(
					&format!("service-{}", managed_url.id),
					&PatchParams::apply(&format!("service-{}", managed_url.id)),
					&Patch::Apply(kubernetes_service),
				)
				.await?
				.status
				.status(500)
				.body(error!(SERVER_ERROR).to_string())?;

			(
				IngressRule {
					host: Some(format!(
						"{}.{}",
						managed_url.sub_domain, domain.name
					)),
					http: Some(HTTPIngressRuleValue {
						paths: vec![HTTPIngressPath {
							backend: IngressBackend {
								service: Some(IngressServiceBackend {
									name: format!("service-{}", managed_url.id),
									port: Some(ServiceBackendPort {
										number: Some(443),
										..ServiceBackendPort::default()
									}),
								}),
								..Default::default()
							},
							path: Some(managed_url.path.to_string()),
							path_type: Some("Prefix".to_string()),
						}],
					}),
				},
				[
					(
						"kubernetes.io/ingress.class".to_string(),
						"nginx".to_string(),
					),
					(
						"nginx.ingress.kubernetes.io/upstream-vhost"
							.to_string(),
						url.clone(),
					),
					(
						"nginx.ingress.kubernetes.io/backend-protocol"
							.to_string(),
						"HTTPS".to_string(),
					),
					(
						"cert-manager.io/issuer".to_string(),
						config.kubernetes.cert_issuer.clone(),
					),
				]
				.into_iter()
				.collect(),
			)
		}
		ManagedUrlType::Redirect { url } => (
			IngressRule {
				host: Some(format!(
					"{}.{}",
					managed_url.sub_domain, domain.name
				)),
				http: Some(HTTPIngressRuleValue {
					paths: vec![HTTPIngressPath {
						backend: IngressBackend {
							..Default::default()
						},
						path: Some(managed_url.path.to_string()),
						path_type: Some("Prefix".to_string()),
					}],
				}),
			},
			[
				(
					"kubernetes.io/ingress.class".to_string(),
					"nginx".to_string(),
				),
				(
					"nginx.ingress.kubernetes.io/temporal-redirect".to_string(),
					url.clone(),
				),
				(
					"cert-manager.io/issuer".to_string(),
					config.kubernetes.cert_issuer.clone(),
				),
			]
			.into_iter()
			.collect(),
		),
	};

	let kubernetes_ingress = Ingress {
		metadata: ObjectMeta {
			name: Some(format!("ingress-{}", managed_url.id)),
			annotations: Some(annotations),
			..ObjectMeta::default()
		},
		spec: Some(IngressSpec {
			rules: Some(vec![ingress]),
			tls: Some(vec![
				if
				/* domain.is_patr_controlled */
				false {
					IngressTLS {
						hosts: Some(vec![format!(
							"{}.{}",
							managed_url.sub_domain, domain.name
						)]),
						secret_name: Some(format!(
							"tls-domain-wildcard-{}",
							domain.name.replace(".", "-")
						)),
					}
				} else {
					IngressTLS {
						hosts: Some(vec![format!(
							"{}.{}",
							managed_url.sub_domain, domain.name
						)]),
						secret_name: Some(format!(
							"tls-url-{}-{}",
							managed_url.sub_domain.replace(".", "-"),
							domain.name.replace(".", "-")
						)),
					}
				},
			]),
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
			&format!("ingress-{}", managed_url.id),
			&PatchParams::apply(&format!("ingress-{}", managed_url.id)),
			&Patch::Apply(kubernetes_ingress),
		)
		.await?
		.status
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;
	log::trace!("request_id: {} - managed URL created", request_id);
	Ok(())
}

pub async fn delete_kubernetes_managed_url(
	workspace_id: &Uuid,
	managed_url_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client = get_kubernetes_config(config).await?;

	let namespace = workspace_id.as_str();
	log::trace!(
		"request_id: {} - deleting service: service-{}",
		request_id,
		managed_url_id
	);

	if service_exists(managed_url_id, kubernetes_client.clone(), namespace)
		.await?
	{
		log::trace!(
			"request_id: {} - service exists as {}",
			request_id,
			managed_url_id
		);

		Api::<Service>::namespaced(kubernetes_client.clone(), namespace)
			.delete(
				&format!("service-{}", managed_url_id),
				&DeleteParams::default(),
			)
			.await?;
		log::trace!(
			"request_id: {} - deployment deleted successfully!",
			request_id
		);
	} else {
		log::trace!(
			"request_id: {} - managed URL doesn't exist as {}",
			request_id,
			managed_url_id
		);
	}

	if ingress_exists(managed_url_id, kubernetes_client.clone(), namespace)
		.await?
	{
		log::trace!(
			"request_id: {} - ingress exists as {}",
			request_id,
			managed_url_id
		);

		Api::<Ingress>::namespaced(kubernetes_client, namespace)
			.delete(
				&format!("ingress-{}", managed_url_id),
				&DeleteParams::default(),
			)
			.await?;
	} else {
		log::trace!(
			"request_id: {} - ingress doesn't exist as {}",
			request_id,
			managed_url_id
		);
	}

	log::trace!(
		"request_id: {} - managed URL deleted successfully!",
		request_id
	);
	Ok(())
}

pub async fn get_container_logs(
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

async fn service_exists(
	service_id: &Uuid,
	kubernetes_client: kube::Client,
	namespace: &str,
) -> Result<bool, KubeError> {
	let service = Api::<Service>::namespaced(kubernetes_client, namespace)
		.get(&format!("service-{}", service_id))
		.await;
	if let Err(KubeError::Api(error)) = service {
		if error.code == 404 {
			return Ok(false);
		} else {
			return Err(KubeError::Api(error));
		}
	}

	Ok(true)
}

async fn ingress_exists(
	managed_url_id: &Uuid,
	kubernetes_client: kube::Client,
	namespace: &str,
) -> Result<bool, KubeError> {
	let ingress = Api::<Ingress>::namespaced(kubernetes_client, namespace)
		.get(&format!("ingress-{}", managed_url_id))
		.await;
	if let Err(KubeError::Api(error)) = ingress {
		if error.code == 404 {
			return Ok(false);
		} else {
			return Err(KubeError::Api(error));
		}
	}

	Ok(true)
}
