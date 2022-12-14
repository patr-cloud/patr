use api_models::utils::Uuid;
use eve_rs::AsError;
use k8s_openapi::api::networking::v1::{
	HTTPIngressPath,
	HTTPIngressRuleValue,
	Ingress,
	IngressBackend,
	IngressRule,
	IngressServiceBackend,
	IngressSpec,
	IngressTLS,
	ServiceBackendPort,
};
use kube::{
	self,
	api::{DeleteParams, Patch, PatchParams},
	core::ObjectMeta,
	Api,
};
use kubernetes::ext_traits::DeleteOpt;

use crate::{
	error,
	service::{infrastructure::kubernetes, KubernetesConfigDetails},
	utils::{settings::Settings, Error},
};

pub async fn update_kubernetes_managed_url(
	workspace_id: &Uuid,
	domain_id: &Uuid,
	sub_domain: &str,
	domain_name: &str,
	urls: Vec<(String, Uuid, i32)>,
	kubeconfig: KubernetesConfigDetails,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client =
		super::get_kubernetes_client(kubeconfig.auth_details).await?;

	let namespace = workspace_id.as_str();
	let (cert_name, cert_host, ingress_name) = if sub_domain == "@" {
		(
			format!("cert-{}", domain_id),
			domain_name.to_owned(),
			format!("ingress-{}", domain_id),
		)
	} else {
		(
			format!("cert-{}-{}", sub_domain, domain_id),
			format!("{}.{}", sub_domain, domain_name),
			format!("ingress-{}-{}", sub_domain, domain_id),
		)
	};

	if urls.is_empty() {
		// no ingress rule is present for this host,
		// so delete the ingress itself
		log::trace!(
			"request_id: {} - no ingress rules present, so deleting {}",
			request_id,
			ingress_name
		);

		Api::<Ingress>::namespaced(kubernetes_client, namespace)
			.delete_opt(&ingress_name, &DeleteParams::default())
			.await?;

		return Ok(());
	}

	log::trace!(
		"request_id: {} - generating managed url configuration",
		request_id
	);

	let annotations = [
		(
			"kubernetes.io/ingress.class".to_string(),
			"nginx".to_string(),
		),
		(
			"cert-manager.io/cluster-issuer".to_string(),
			config.kubernetes.cert_issuer_http.clone(),
		),
	]
	.into_iter()
	.collect();

	let ingress_rules = urls
		.into_iter()
		.map(|(path, deployment_id, port)| IngressRule {
			host: Some(cert_host.clone()),
			http: Some(HTTPIngressRuleValue {
				paths: vec![HTTPIngressPath {
					backend: IngressBackend {
						service: Some(IngressServiceBackend {
							name: format!("service-{}", deployment_id),
							port: Some(ServiceBackendPort {
								number: Some(port),
								..ServiceBackendPort::default()
							}),
						}),
						..Default::default()
					},
					path: Some(path),
					path_type: Some("Prefix".to_string()),
				}],
			}),
		})
		.collect();

	let kubernetes_ingress = Ingress {
		metadata: ObjectMeta {
			name: Some(ingress_name.clone()),
			annotations: Some(annotations),
			..ObjectMeta::default()
		},
		spec: Some(IngressSpec {
			rules: Some(ingress_rules),
			tls: Some(vec![IngressTLS {
				hosts: Some(vec![cert_host]),
				secret_name: Some(cert_name),
			}]),
			..IngressSpec::default()
		}),
		..Ingress::default()
	};

	// Create the ingress defined above
	log::trace!("request_id: {} - creating ingress", request_id);

	Api::<Ingress>::namespaced(kubernetes_client, namespace)
		.patch(
			&ingress_name,
			&PatchParams::apply(&ingress_name),
			&Patch::Apply(kubernetes_ingress),
		)
		.await?
		.status
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	log::trace!(
		"request_id: {} - kubernetes managed URL updated",
		request_id
	);
	Ok(())
}
