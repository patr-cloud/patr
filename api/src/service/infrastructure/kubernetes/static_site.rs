use std::collections::BTreeMap;

use api_models::{
	models::workspace::infrastructure::static_site::StaticSiteDetails,
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
	api::{DeleteParams, Patch, PatchParams},
	core::ObjectMeta,
	Api,
};

use crate::{
	error,
	service::infrastructure::kubernetes,
	utils::{settings::Settings, Error},
};

pub async fn update_kubernetes_static_site(
	workspace_id: &Uuid,
	static_site_id: &Uuid,
	_static_site_details: &StaticSiteDetails,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client = super::get_kubernetes_config(config).await?;
	// new name for the docker image

	let namespace = workspace_id.as_str();
	log::trace!(
		"request_id: {} - generating deployment configuration",
		request_id
	);

	if !kubernetes::secret_exists(
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

	let kubernetes_service = Service {
		metadata: ObjectMeta {
			name: Some(format!("service-{}", static_site_id)),
			..ObjectMeta::default()
		},
		spec: Some(ServiceSpec {
			type_: Some("ExternalName".to_string()),
			external_name: Some(
				config.kubernetes.static_site_proxy_service.to_string(),
			),
			ports: Some(vec![ServicePort {
				port: 80,
				name: Some("http".to_string()),
				protocol: Some("TCP".to_string()),
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
			&format!("service-{}", static_site_id),
			&PatchParams::apply(&format!("service-{}", static_site_id)),
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
		format!("{}.patr.cloud", static_site_id),
	);

	annotations.insert(
		"cert-manager.io/cluster-issuer".to_string(),
		config.kubernetes.cert_issuer_dns.clone(),
	);
	let ingress_rule = vec![IngressRule {
		host: Some(format!("{}.patr.cloud", static_site_id)),
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
		hosts: Some(vec!["*.patr.cloud".to_string(), "patr.cloud".to_string()]),
		secret_name: None,
	}];
	log::trace!(
		"request_id: {} - creating https certificates for domain",
		request_id
	);
	let kubernetes_ingress = Ingress {
		metadata: ObjectMeta {
			name: Some(format!("ingress-{}", static_site_id)),
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
			&format!("ingress-{}", static_site_id),
			&PatchParams::apply(&format!("ingress-{}", static_site_id)),
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
		static_site_id
	);
	Ok(())
}

pub async fn delete_kubernetes_static_site(
	workspace_id: &Uuid,
	static_site_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client = super::get_kubernetes_config(config).await?;

	let namespace = workspace_id.as_str();
	log::trace!(
		"request_id: {} - deleting service: service-{}",
		request_id,
		static_site_id
	);

	if super::service_exists(
		static_site_id,
		kubernetes_client.clone(),
		namespace,
	)
	.await?
	{
		log::trace!(
			"request_id: {} - static site service exists as service-{}",
			request_id,
			static_site_id
		);

		Api::<Service>::namespaced(kubernetes_client.clone(), namespace)
			.delete(
				&format!("service-{}", static_site_id),
				&DeleteParams::default(),
			)
			.await?;
	} else {
		log::trace!(
			"request_id: {} - static site doesn't exist as service-{} in the namespace: {}",
			request_id,
			static_site_id,
			namespace,
		);
	}

	if super::ingress_exists(
		static_site_id,
		kubernetes_client.clone(),
		namespace,
	)
	.await?
	{
		log::trace!(
			"request_id: {} - ingress exists as ingress-{}",
			request_id,
			static_site_id
		);
		Api::<Ingress>::namespaced(kubernetes_client, namespace)
			.delete(
				&format!("ingress-{}", static_site_id),
				&DeleteParams::default(),
			)
			.await?;
	} else {
		log::trace!(
			"request_id: {} - static site ingress doesn't exist as ingress-{} in the namespace: {}",
			request_id,
			static_site_id,
			namespace,
		);
	}

	log::trace!(
		"request_id: {} - static site deleted successfully!",
		request_id
	);
	Ok(())
}
