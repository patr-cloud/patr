use std::collections::BTreeMap;

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
	api::{Patch, PatchParams},
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
};
use uuid::Uuid;

use crate::{
	db,
	error,
	utils::{settings::Settings, Error},
	Database,
};

pub async fn update_static_site(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &[u8],
	config: &Settings,
) -> Result<(), Error> {
	let kubernetes_client = get_kubernetes_config(config).await?;
	let request_id = Uuid::new_v4();
	// TODO: remove this once DTO part is complete
	let static_site = db::get_static_site_by_id(connection, static_site_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let static_site_id_string = hex::encode(&static_site_id);
	// new name for the docker image

	let namespace = "default";
	log::trace!(
		"request_id: {} - generating deployment configuration",
		request_id
	);

	let kubernetes_service = Service {
		metadata: ObjectMeta {
			name: Some(format!("service-{}", &static_site_id_string)),
			..ObjectMeta::default()
		},
		spec: Some(ServiceSpec {
			type_: Some("ExternalName".to_string()),
			external_name: Some(format!(
				"{}.{}",
				config.s3.bucket,
				config.s3.endpoint.to_string()
			)),
			ports: Some(vec![ServicePort {
				port: 80,
				target_port: Some(IntOrString::Int(80)),
				name: Some("http".to_owned()),
				protocol: Some("TCP".to_string()),
				..ServicePort::default()
			}]),
			..ServiceSpec::default()
		}),
		..Service::default()
	};
	// Create the service defined above
	log::trace!("request_id: {} - creating ExternalName service", request_id);
	let service_api: Api<Service> =
		Api::namespaced(kubernetes_client.clone(), namespace);
	service_api
		.patch(
			&format!("service-{}", &static_site_id_string),
			&PatchParams::apply(&format!("service-{}", &static_site_id_string)),
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
	annotations.insert(
		"nginx.ingress.kubernetes.io/upstream-vhost".to_string(),
		format!("{}.{}", config.s3.bucket, config.s3.endpoint.to_string()),
	);
	annotations.insert(
		"cert-manager.io/issuer".to_string(),
		"letsencrypt-prod".to_string(),
	);
	annotations.insert(
		"nginx.ingress.kubernetes.io/rewrite-target".to_string(),
		format!("{}/index.html", static_site_id_string),
	);
	annotations.insert(
		"nginx.ingress.kubernetes.io/use-regex".to_string(),
		"true".to_string(),
	);
	let custom_domain_rule = if let Some(domain) =
		static_site.domain_name.clone()
	{
		log::trace!("request_id: {} - custom domain present, adding domain details to the ingress", request_id);
		annotations.insert(
			"nginx.ingress.kubernetes.io/proxy-redirect-from".to_string(),
			domain.clone(),
		);
		annotations.insert(
			"nginx.ingress.kubernetes.io/proxy-redirect-to".to_string(),
			format!("{}.patr.cloud", static_site_id_string),
		);
		vec![
			IngressRule {
				host: Some(format!("{}.patr.cloud", static_site_id_string)),
				http: Some(HTTPIngressRuleValue {
					paths: vec![HTTPIngressPath {
						backend: IngressBackend {
							service: Some(IngressServiceBackend {
								name: format!(
									"service-{}",
									&static_site_id_string
								),
								port: Some(ServiceBackendPort {
									number: Some(80),
									name: Some("http".to_owned()),
								}),
							}),
							..IngressBackend::default()
						},
						path: Some("/".to_string()),
						path_type: Some("Prefix".to_string()),
					}],
				}),
			},
			IngressRule {
				host: Some(domain),
				http: Some(HTTPIngressRuleValue {
					paths: vec![HTTPIngressPath {
						backend: IngressBackend {
							service: Some(IngressServiceBackend {
								name: format!(
									"service-{}",
									&static_site_id_string
								),
								port: Some(ServiceBackendPort {
									number: Some(80),
									name: Some("http".to_owned()),
								}),
							}),
							..IngressBackend::default()
						},
						path: Some("/".to_string()),
						path_type: Some("Prefix".to_string()),
					}],
				}),
			},
		]
	} else {
		vec![IngressRule {
			host: Some(format!("{}.patr.cloud", static_site_id_string)),
			http: Some(HTTPIngressRuleValue {
				paths: vec![HTTPIngressPath {
					backend: IngressBackend {
						service: Some(IngressServiceBackend {
							name: format!("service-{}", &static_site_id_string),
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
		}]
	};
	let custom_domain_tls = if let Some(domain) = static_site.domain_name {
		log::trace!(
			"request_id: {} - adding custom domain config to ingress",
			request_id
		);
		vec![
			IngressTLS {
				hosts: Some(vec![format!(
					"{}.patr.cloud",
					static_site_id_string
				)]),
				secret_name: Some(format!("tls-{}", &static_site_id_string)),
			},
			IngressTLS {
				hosts: Some(vec![domain]),
				secret_name: Some(format!(
					"custom-tls-{}",
					&static_site_id_string
				)),
			},
		]
	} else {
		log::trace!(
			"request_id: {} - adding patr domain config to ingress",
			request_id
		);
		vec![IngressTLS {
			hosts: Some(vec![format!("{}.patr.cloud", static_site_id_string)]),
			secret_name: Some(format!("tls-{}", &static_site_id_string)),
		}]
	};
	log::trace!(
		"request_id: {} - creating https certificates for domain",
		request_id
	);
	let kubernetes_ingress: Ingress = Ingress {
		metadata: ObjectMeta {
			name: Some(format!("ingress-{}", &static_site_id_string)),
			annotations: Some(annotations),
			..ObjectMeta::default()
		},
		spec: Some(IngressSpec {
			rules: Some(custom_domain_rule),
			tls: Some(custom_domain_tls),
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
			&format!("ingress-{}", &static_site_id_string),
			&PatchParams::apply(&format!("ingress-{}", &static_site_id_string)),
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
		static_site_id_string
	);
	Ok(())
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
