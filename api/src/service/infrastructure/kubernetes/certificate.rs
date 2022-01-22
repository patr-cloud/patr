use api_models::utils::Uuid;
use k8s_openapi::{self, api::core::v1::Secret};
use kube::{
	self,
	api::{DeleteParams, Patch, PatchParams},
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
	config: &Settings,
) -> Result<(), Error> {
	let kubernetes_client = super::get_kubernetes_config(config).await?;

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
	let certificate_data = json!(
		{
			"spec": {
				"secretName": secret_name,
				"dnsNames": domain_list,
				"issuerRef": {
					"name": config.kubernetes.cert_issuer,
					"kind": "ClusterIssuer",
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
			name: Some(certificate_name.to_string()),
			..ObjectMeta::default()
		},
		data: certificate_data,
	};

	let namespace = workspace_id.as_str();

	let _ = Api::<DynamicObject>::namespaced_with(
		kubernetes_client,
		namespace,
		&certificate_resource,
	)
	.patch(
		certificate_name,
		&PatchParams::apply(certificate_name),
		&Patch::Apply(&certificate),
	)
	.await?;

	Ok(())
}

pub async fn delete_certificates_for_domain(
	workspace_id: &Uuid,
	certificate_name: &str,
	secret_name: &str,
	config: &Settings,
) -> Result<(), Error> {
	let kubernetes_client = super::get_kubernetes_config(config).await?;

	let namespace = workspace_id.as_str();

	// delete secret and then certificate
	if super::secret_exists(secret_name, kubernetes_client.clone(), namespace)
		.await?
	{
		Api::<Secret>::namespaced(kubernetes_client.clone(), namespace)
			.delete(secret_name, &DeleteParams::default())
			.await?;
	}

	let certificate_resource = ApiResource {
		group: "cert-manager.io".to_string(),
		version: "v1".to_string(),
		api_version: "cert-manager.io/v1".to_string(),
		kind: "certificate".to_string(),
		plural: "certificates".to_string(),
	};

	Api::<DynamicObject>::namespaced_with(
		kubernetes_client,
		namespace,
		&certificate_resource,
	)
	.delete(certificate_name, &DeleteParams::default())
	.await?;

	Ok(())
}
