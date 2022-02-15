use api_models::utils::Uuid;
use k8s_openapi::{self, api::core::v1::Secret};
use kube::{
	self,
	api::{DeleteParams, PostParams},
	core::{ApiResource, DynamicObject, ObjectMeta, TypeMeta},
	Api,
};
use serde_json::json;

use crate::utils::{settings::Settings, Error};

// TODO: add logs
pub async fn create_certificates(
	workspace_id: &Uuid,
	certificate_name: &str,
	secret_name: &str,
	domain_list: Vec<String>,
	is_internal: bool,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!("request_id: {} - Creating certificates", request_id);
	let kubernetes_client = super::get_kubernetes_config(config).await?;

	log::trace!(
		"request_id: {} - Checking if certificate exists",
		request_id
	);
	if super::certificate_exists(
		certificate_name,
		kubernetes_client.clone(),
		workspace_id.as_str(),
	)
	.await?
	{
		return Ok(());
	}

	let certificate_resource = ApiResource {
		group: "cert-manager.io".to_string(),
		version: "v1".to_string(),
		api_version: "cert-manager.io/v1".to_string(),
		kind: "certificate".to_string(),
		plural: "certificates".to_string(),
	};

	// TODO: use yaml raw string to be converted in to value
	let certificate_data = if is_internal {
		json!(
			{
				"spec": {
					"secretName": secret_name,
					"dnsNames": domain_list,
					"issuerRef": {
						"name": config.kubernetes.cert_issuer_dns,
						"kind": "ClusterIssuer",
						"group": "cert-manager.io"
					},
				}
			}
		)
	} else {
		json!(
			{
				"spec": {
					"secretName": secret_name,
					"dnsNames": domain_list,
					"issuerRef": {
						"name": config.kubernetes.cert_issuer_http,
						"kind": "ClusterIssuer",
						"group": "cert-manager.io"
					},
				}
			}
		)
	};

	let certificate = DynamicObject {
		types: Some(TypeMeta {
			api_version: "cert-manager.io/v1".to_string(),
			kind: "certificate".to_string(),
		}),
		metadata: ObjectMeta {
			name: Some(certificate_name.to_string()),
			..ObjectMeta::default()
		},
		data: certificate_data,
	};

	let namespace = workspace_id.as_str();

	log::trace!(
		"request_id: {} - Creating certificate in Kubernetes",
		request_id
	);
	let _ = Api::<DynamicObject>::namespaced_with(
		kubernetes_client,
		namespace,
		&certificate_resource,
	)
	.create(&PostParams::default(), &certificate)
	.await?;

	log::trace!("request_id: {} - Certificate created", request_id);
	Ok(())
}

pub async fn delete_certificates_for_domain(
	workspace_id: &Uuid,
	certificate_name: &str,
	secret_name: &str,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!("request_id: {} - Deleting certificates", request_id);
	let kubernetes_client = super::get_kubernetes_config(config).await?;

	let namespace = workspace_id.as_str();

	// delete secret and then certificate
	log::trace!("request_id: {} - Deleting secret", request_id);
	if super::secret_exists(secret_name, kubernetes_client.clone(), namespace)
		.await?
	{
		Api::<Secret>::namespaced(kubernetes_client.clone(), namespace)
			.delete(secret_name, &DeleteParams::default())
			.await?;
	}

	if !super::certificate_exists(
		certificate_name,
		kubernetes_client.clone(),
		namespace,
	)
	.await?
	{
		return Ok(());
	}

	let certificate_resource = ApiResource {
		group: "cert-manager.io".to_string(),
		version: "v1".to_string(),
		api_version: "cert-manager.io/v1".to_string(),
		kind: "certificate".to_string(),
		plural: "certificates".to_string(),
	};

	log::trace!(
		"request_id: {} - Deleting certificate in Kubernetes",
		request_id
	);
	Api::<DynamicObject>::namespaced_with(
		kubernetes_client,
		namespace,
		&certificate_resource,
	)
	.delete(certificate_name, &DeleteParams::default())
	.await?;

	Ok(())
}
