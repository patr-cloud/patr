use api_models::utils::Uuid;
use k8s_openapi::api::{
	core::v1::{Service, ServiceSpec},
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
};
use kube::{
	api::{Patch, PatchParams},
	core::ObjectMeta,
	Api,
};

use crate::{service::KubernetesConfigDetails, utils::Error};

pub enum ResourceType {
	StaticSite { id: Uuid },
	Deployment { id: Uuid, ports: Vec<u16> },
}

pub enum ResourceAction {
	Stopped,
	Deleted,
}

pub async fn redirect_to_resource_down_page(
	workspace_id: &Uuid,
	resource_type: &ResourceType,
	action: ResourceAction,
	kubeconfig: KubernetesConfigDetails,
	cert_issuer_dns: &str,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client =
		super::get_kubernetes_client(kubeconfig.auth_details).await?;

	let namespace = workspace_id.as_str();
	let resource_id = match &resource_type {
		ResourceType::StaticSite { id } => id,
		ResourceType::Deployment { id, .. } => id,
	};
	log::trace!("request_id: {request_id} - Updating resource as down in ingress for `{resource_id}`");

	let resource_down_host = match action {
		ResourceAction::Stopped => "stopped.patr.cloud",
		ResourceAction::Deleted => "deleted.patr.cloud",
	};

	let service_name = format!("service-{resource_id}");
	let kubernetes_service = Service {
		metadata: ObjectMeta {
			name: Some(service_name.clone()),
			..ObjectMeta::default()
		},
		spec: Some(ServiceSpec {
			type_: Some("ExternalName".to_string()),
			external_name: Some(resource_down_host.to_owned()),
			..ServiceSpec::default()
		}),
		..Service::default()
	};
	Api::<Service>::namespaced(kubernetes_client.clone(), namespace)
		.patch(
			&service_name,
			&PatchParams::apply(&service_name),
			&Patch::Apply(kubernetes_service),
		)
		.await?;

	let annotations = [
		("kubernetes.io/ingress.class", "nginx"),
		("nginx.ingress.kubernetes.io/backend-protocol", "https"),
		("cert-manager.io/cluster-issuer", cert_issuer_dns),
		(
			"nginx.ingress.kubernetes.io/upstream-vhost",
			resource_down_host,
		),
	]
	.into_iter()
	.map(|(k, v)| (k.to_owned(), v.to_owned()))
	.collect();

	let resource_hosts = match resource_type {
		ResourceType::StaticSite { id } => {
			vec![format!("{id}.patr.cloud")]
		}
		ResourceType::Deployment { id, ports } => ports
			.iter()
			.map(|port| format!("{port}-{id}.patr.cloud"))
			.collect(),
	};

	let ingress_rules = resource_hosts
		.into_iter()
		.map(|resource_host| IngressRule {
			host: Some(resource_host),
			http: Some(HTTPIngressRuleValue {
				paths: vec![HTTPIngressPath {
					path: Some("/".to_string()),
					path_type: Some("Prefix".to_string()),
					backend: IngressBackend {
						service: Some(IngressServiceBackend {
							name: service_name.clone(),
							port: Some(ServiceBackendPort {
								number: Some(443),
								..ServiceBackendPort::default()
							}),
						}),
						..Default::default()
					},
				}],
			}),
		})
		.collect();

	let ingress_name = format!("ingress-{resource_id}");
	let kubernetes_ingress = Ingress {
		metadata: ObjectMeta {
			name: Some(ingress_name.clone()),
			annotations: Some(annotations),
			..ObjectMeta::default()
		},
		spec: Some(IngressSpec {
			rules: Some(ingress_rules),
			tls: Some(vec![IngressTLS {
				hosts: Some(vec![
					"*.patr.cloud".to_string(),
					"patr.cloud".to_string(),
				]),
				secret_name: None,
			}]),
			..IngressSpec::default()
		}),
		..Ingress::default()
	};
	Api::<Ingress>::namespaced(kubernetes_client, workspace_id.as_str())
		.patch(
			&ingress_name,
			&PatchParams::apply(&ingress_name),
			&Patch::Apply(kubernetes_ingress),
		)
		.await?;

	log::trace!("request_id: {request_id} - Updated resource as down in ingress for `{resource_id}`");
	Ok(())
}
