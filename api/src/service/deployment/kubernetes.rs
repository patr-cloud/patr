use std::collections::BTreeMap;

use eve_rs::AsError;
use k8s_openapi::{
	api::{
		core::v1::{Secret, Service, ServicePort, ServiceSpec},
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
	api::{DeleteParams, Patch, PatchParams},
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
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client = get_kubernetes_config(config).await?;
	// TODO: remove this once DTO part is complete
	let _ = db::get_static_site_by_id(connection, static_site_id)
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

	let mut selector = BTreeMap::new();
	selector.insert("app".to_string(), "static-sites-proxy".to_string());

	let kubernetes_service = Service {
		metadata: ObjectMeta {
			name: Some(format!("service-{}", &static_site_id_string)),
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
			&format!("service-{}", &static_site_id_string),
			&PatchParams::apply(&format!("service-{}", &static_site_id_string)),
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
		format!("{}.patr.cloud", static_site_id_string),
	);

	// TODO convert this into config
	annotations.insert(
		"cert-manager.io/issuer".to_string(),
		config.cert_issuer.clone(),
	);
	let ingress_rule = vec![IngressRule {
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
	}];

	log::trace!(
		"request_id: {} - adding patr domain config to ingress",
		request_id
	);
	let patr_domain_tls = vec![IngressTLS {
		hosts: Some(vec![format!("{}.patr.cloud", static_site_id_string)]),
		secret_name: Some(format!("tls-{}", &static_site_id_string)),
	}];
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

pub async fn delete_static_site(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &[u8],
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client = get_kubernetes_config(config).await?;

	let _ = db::get_static_site_by_id(connection, static_site_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let static_site_id_string = hex::encode(&static_site_id);
	// new name for the docker image

	let namespace = "default";
	log::trace!(
		"request_id: {} - deleting service: service-{}",
		request_id,
		static_site_id_string
	);

	if !service_exists(static_site_id, kubernetes_client.clone(), namespace)
		.await?
	{
		log::trace!(
			"request_id: {} - App doesn't exist as {}",
			request_id,
			static_site_id_string
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
			static_site_id_string
		);

		let _service_api =
			Api::<Service>::namespaced(kubernetes_client.clone(), namespace)
				.delete(
					&format!("service-{}", &static_site_id_string),
					&DeleteParams::default(),
				)
				.await?;
		let _ingress_api =
			Api::<Ingress>::namespaced(kubernetes_client, namespace)
				.delete(
					&format!("ingress-{}", &static_site_id_string),
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

pub async fn delete_tls_certificate(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &[u8],
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client = get_kubernetes_config(config).await?;

	// TODO: replace this with static site object
	let static_site = db::get_static_site_by_id(connection, static_site_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	log::trace!("request_id: {} - deleting tls certificate", request_id);

	log::trace!("request_id: {} - checking if secret exists", request_id);
	if !secret_exists(
		&format!("tls-{}", &hex::encode(static_site_id)),
		kubernetes_client.clone(),
		"default",
	)
	.await?
	{
		log::trace!(
			"request_id: {} - tls certificate doesn't exist",
			request_id
		);
		log::trace!(
			"request_id: {} - tls certificate deleted successfully!",
			request_id
		);
		return Ok(());
	}

	let _secret_api =
		Api::<Secret>::namespaced(kubernetes_client.clone(), "default")
			.delete(
				&format!("tls-{}", &hex::encode(static_site_id)),
				&DeleteParams::default(),
			)
			.await?;

	log::trace!(
		"request_id: {} - tls certificate deleted successfully!",
		request_id
	);

	if let Some(custom_domain) = static_site.domain_name {
		log::trace!("request_id: {} - checking if secret exists", request_id);
		if !secret_exists(
			&format!("tls-{}", &hex::encode(static_site_id)),
			kubernetes_client.clone(),
			"default",
		)
		.await?
		{
			log::trace!(
				"request_id: {} - tls certificate doesn't exist",
				request_id
			);
			log::trace!(
				"request_id: {} - custom tls certificate deleted successfully!",
				request_id
			);

			return Ok(());
		}
		let _secret_api =
			Api::<Secret>::namespaced(kubernetes_client, "default")
				.delete(
					&format!("Custom-tls-{}", &custom_domain),
					&DeleteParams::default(),
				)
				.await?;

		log::trace!(
			"request_id: {} - custom tls certificate deleted successfully!",
			request_id
		);
	}

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

async fn service_exists(
	static_site_id: &[u8],
	kubernetes_client: kube::Client,
	namespace: &str,
) -> Result<bool, Error> {
	let deployment_app =
		Api::<Service>::namespaced(kubernetes_client, namespace)
			.get(&format!("service-{}", &hex::encode(&static_site_id)))
			.await;
	if deployment_app.is_err() {
		// TODO: catch the not found error here
		return Ok(false);
	}
	Ok(true)
}

async fn secret_exists(
	secret_name: &str,
	kubernetes_client: kube::Client,
	namespace: &str,
) -> Result<bool, Error> {
	let secret_app = Api::<Secret>::namespaced(kubernetes_client, namespace)
		.get(secret_name)
		.await;
	if secret_app.is_err() {
		// TODO: catch the not found error here
		return Ok(false);
	}
	Ok(true)
}
