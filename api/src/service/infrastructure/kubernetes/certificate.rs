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
			annotations: None,
			cluster_name: None,
			creation_timestamp: None,
			deletion_grace_period_seconds: None,
			deletion_timestamp: None,
			finalizers: None,
			generate_name: None,
			generation: None,
			labels: None,
			managed_fields: None,
			name: Some(certificate_name.to_string()),
			namespace: None,
			owner_references: None,
			resource_version: None,
			self_link: None,
			uid: None,
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
		&PatchParams::default(),
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

	Api::<Secret>::namespaced(kubernetes_client.clone(), namespace)
		.delete(secret_name, &DeleteParams::default())
		.await?;

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
