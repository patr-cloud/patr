use std::ops::DerefMut;

use api_models::{
	models::workspace::infrastructure::managed_urls::{
		ManagedUrl,
		ManagedUrlType,
	},
	utils::Uuid,
};
use eve_rs::AsError;
use k8s_openapi::{
	api::{
		core::v1::{Service, ServicePort, ServiceSpec},
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
	apimachinery::pkg::util::intstr::IntOrString,
};
use kube::{
	self,
	api::{DeleteParams, Patch, PatchParams},
	core::ObjectMeta,
	Api,
};

use crate::{
	db,
	error,
	service,
	utils::{settings::Settings, Error},
};

pub async fn update_kubernetes_managed_url(
	workspace_id: &Uuid,
	managed_url: &ManagedUrl,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client = super::get_kubernetes_config(config).await?;

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
					"cert-manager.io/cluster-issuer".to_string(),
					if domain.is_ns_internal() {
						config.kubernetes.cert_issuer_dns.clone()
					} else {
						config.kubernetes.cert_issuer_http.clone()
					},
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
					"cert-manager.io/cluster-issuer".to_string(),
					if domain.is_ns_internal() {
						config.kubernetes.cert_issuer_dns.clone()
					} else {
						config.kubernetes.cert_issuer_http.clone()
					},
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
						"cert-manager.io/cluster-issuer".to_string(),
						if domain.is_ns_internal() {
							config.kubernetes.cert_issuer_dns.clone()
						} else {
							config.kubernetes.cert_issuer_http.clone()
						},
					),
				]
				.into_iter()
				.collect(),
			)
		}
		ManagedUrlType::Redirect { url } => {
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
						"nginx.ingress.kubernetes.io/temporal-redirect"
							.to_string(),
						url.clone(),
					),
					(
						"cert-manager.io/cluster-issuer".to_string(),
						if domain.is_ns_internal() {
							config.kubernetes.cert_issuer_dns.clone()
						} else {
							config.kubernetes.cert_issuer_http.clone()
						},
					),
				]
				.into_iter()
				.collect(),
			)
		}
	};

	let kubernetes_ingress = Ingress {
		metadata: ObjectMeta {
			name: Some(format!("ingress-{}", managed_url.id)),
			annotations: Some(annotations),
			..ObjectMeta::default()
		},
		spec: Some(IngressSpec {
			rules: Some(vec![ingress]),
			tls: Some(vec![IngressTLS {
				hosts: Some(vec![format!(
					"{}.{}",
					managed_url.sub_domain, domain.name
				)]),
				secret_name: Some(format!(
					"tls-{}",
					if domain.is_ns_internal() {
						&domain.id
					} else {
						&managed_url.id
					}
				)),
			}]),
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
	let kubernetes_client = super::get_kubernetes_config(config).await?;

	let namespace = workspace_id.as_str();
	log::trace!(
		"request_id: {} - deleting service: service-{}",
		request_id,
		managed_url_id
	);

	if super::service_exists(
		managed_url_id,
		kubernetes_client.clone(),
		namespace,
	)
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
			"request_id: {} - managed URL service doesn't exist as service-{}",
			request_id,
			managed_url_id
		);
	}

	if super::ingress_exists(
		managed_url_id,
		kubernetes_client.clone(),
		namespace,
	)
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
			"request_id: {} - managed URL ingress doesn't exist as ingress-{}",
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
