use api_models::utils::Uuid;
use k8s_openapi::{
	self,
	api::core::v1::{Namespace, NamespaceSpec},
};
use kube::{self, api::PostParams, core::ObjectMeta, Api};

use crate::utils::{settings::Settings, Error};

pub async fn create_kubernetes_namespace(
	namespace_name: &str,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let client = super::get_kubernetes_config(config).await?;

	log::trace!("request_id: {} - creating namespace", request_id);
	let kubernetes_namespace = Namespace {
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
			name: Some(namespace_name.to_string()),
			namespace: None,
			owner_references: None,
			resource_version: None,
			self_link: None,
			uid: None,
		},
		spec: Some(NamespaceSpec { finalizers: None }),
		status: None,
	};
	let namespace_api = Api::<Namespace>::all(client);
	namespace_api
		.create(&PostParams::default(), &kubernetes_namespace)
		.await?;
	log::trace!("request_id: {} - namespace created", request_id);

	Ok(())
}
