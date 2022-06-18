use std::{ops::DerefMut, time::Duration};

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
use redis::AsyncCommands;
use tokio::time;

use crate::{
	db::{self, WorkspaceDomain},
	error,
	service::{self, infrastructure::kubernetes},
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
		"request_id: {} - generating managed url configuration",
		request_id
	);

	let domain = db::get_workspace_domain_by_id(
		service::get_app().database.acquire().await?.deref_mut(),
		&managed_url.domain_id,
	)
	.await?
	.status(500)?;

	let secret_name = format!(
		"tls-{}",
		if domain.is_ns_internal() {
			&domain.id
		} else {
			&managed_url.id
		}
	);

	if !kubernetes::secret_exists(
		&secret_name,
		kubernetes_client.clone(),
		namespace,
	)
	.await?
	{
		time::sleep(Duration::from_secs(30)).await;
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	}

	let host = if managed_url.sub_domain == "@" {
		domain.name.clone()
	} else {
		format!("{}.{}", managed_url.sub_domain, domain.name)
	};

	let (ingress, annotations) = match &managed_url.url_type {
		ManagedUrlType::ProxyDeployment {
			deployment_id,
			port,
		} => (
			IngressRule {
				host: Some(host.clone()),
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
						path: Some(format!(
							"{}/{}",
							managed_url.path, ".well-known/patr/cert-file"
						)),
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
					host.clone(),
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
				host: Some(host.clone()),
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
						path: Some(format!(
							"{}/{}",
							managed_url.path, ".well-known/patr/cert-file"
						)),
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
					format!("{}.patr.cloud", static_site_id),
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
					host: Some(host.clone()),
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
							path: Some(format!(
								"{}/{}",
								managed_url.path, ".well-known/patr/cert-file"
							)),
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
					host: Some(host.clone()),
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
							path: Some(format!(
								"{}/{}",
								managed_url.path, ".well-known/patr/cert-file"
							)),
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
						format!("https://{}", url),
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

	let secret_name = format!(
		"tls-{}",
		if domain.is_ns_internal() {
			&domain.id
		} else {
			&managed_url.id
		}
	);
	let secret_exists = super::secret_exists(
		&secret_name,
		kubernetes_client.clone(),
		namespace,
	)
	.await?;

	let kubernetes_ingress = Ingress {
		metadata: ObjectMeta {
			name: Some(format!("ingress-{}", managed_url.id)),
			annotations: Some(annotations),
			..ObjectMeta::default()
		},
		spec: Some(IngressSpec {
			rules: Some(vec![ingress]),
			tls: if secret_exists {
				Some(vec![IngressTLS {
					hosts: if domain.is_ns_internal() {
						Some(vec![
							format!("*.{}", domain.name),
							domain.name.clone(),
						])
					} else {
						Some(vec![host.clone()])
					},
					secret_name: Some(secret_name),
				}])
			} else {
				None
			},
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

	// Randomly generate a secret content for the TLS certificate
	let verification_secret = Uuid::new_v4();

	// Put content and ingress in the Redis
	let app = service::get_app();
	let mut redis = app.redis.clone();

	redis
		.set(
			format!("verfification-{}", managed_url.id),
			verification_secret.to_string(),
		)
		.await?;

	verify_managed_url(
		workspace_id,
		&domain,
		managed_url,
		&host,
		verification_secret.as_str(),
		config,
		request_id,
	)
	.await?;
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

pub async fn verify_managed_url(
	workspace_id: &Uuid,
	domain: &WorkspaceDomain,
	managed_url: &ManagedUrl,
	host: &str,
	verification_secret: &str,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client = super::get_kubernetes_config(config).await?;
	let namespace = workspace_id.as_str();

	let secret_name = format!(
		"tls-{}",
		if domain.is_ns_internal() {
			&domain.id
		} else {
			&managed_url.id
		}
	);
	let secret_exists = super::secret_exists(
		&secret_name,
		kubernetes_client.clone(),
		namespace,
	)
	.await?;

	let (ingress, annotations) = (
			IngressRule {
				host: Some(host.to_string()),
				http: Some(HTTPIngressRuleValue {
					paths: vec![HTTPIngressPath {
						backend: IngressBackend {
							service: Some(IngressServiceBackend {
								name: format!("service-verify-{}", managed_url.id),
								port: Some(ServiceBackendPort {
									number: Some(80),
									..ServiceBackendPort::default()
								}),
							}),
							..Default::default()
						},
						path: Some("/.well-known/patr/cert-file".to_string()),
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
					format!("http://localhost:3006/workspace/{}/infrastructure/managed-url/{}/verify", workspace_id, managed_url.id),
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
	);

	let kubernetes_ingress = Ingress {
		metadata: ObjectMeta {
			name: Some(format!("ingress-verify-{}", managed_url.id)),
			annotations: Some(annotations),
			..ObjectMeta::default()
		},
		spec: Some(IngressSpec {
			rules: Some(vec![ingress]),
			tls: if secret_exists {
				Some(vec![IngressTLS {
					hosts: if domain.is_ns_internal() {
						Some(vec![
							format!("*.{}", domain.name),
							domain.name.clone(),
						])
					} else {
						Some(vec![host.to_string()])
					},
					secret_name: Some(secret_name),
				}])
			} else {
				None
			},
			..IngressSpec::default()
		}),
		..Ingress::default()
	};
	// Create the ingress defined above
	log::trace!(
		"request_id: {} - creating verification string ingress",
		request_id
	);
	let ingress_api: Api<Ingress> =
		Api::namespaced(kubernetes_client, namespace);
	ingress_api
		.patch(
			&format!("ingress-verify-{}", managed_url.id),
			&PatchParams::apply(&format!("ingress-verify-{}", managed_url.id)),
			&Patch::Apply(kubernetes_ingress),
		)
		.await?;

	let verification_response =
		reqwest::get(format!("https://{}/.well-known/patr/cert-file", host))
			.await?
			.text()
			.await?;

	if verification_response != verification_secret {
		log::error!(
			"request_id: {} - verification string mismatch, scheduling for re-verification",
			request_id
		);
		// Schedule a job to verify the managed URL again
		ingress_api
			.delete(
				&format!("ingress-verify-{}", managed_url.id),
				&DeleteParams::default(),
			)
			.await?;

		// Verification string mismatch
		Ok(())
	} else {
		// Create certificate for the managed URL
		log::error!("request_id: {} - Managed URL verified", request_id);
		ingress_api
			.delete(
				&format!("ingress-verify-{}", managed_url.id),
				&DeleteParams::default(),
			)
			.await?;

		// Create certificate for the managed URL

		Ok(())
	}
}
