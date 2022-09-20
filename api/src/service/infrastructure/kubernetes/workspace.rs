use api_models::utils::Uuid;
use k8s_openapi::{self, api::core::v1::Namespace};
use kube::{
	self,
	api::{DeleteParams, PostParams, PropagationPolicy},
	core::ObjectMeta,
	Api,
};

use crate::{
	service::{ext_traits::DeleteOpt, KubernetesConfig},
	utils::Error,
};

pub async fn create_kubernetes_namespace(
	namespace_name: &str,
	kubeconfig: KubernetesConfig,
	request_id: &Uuid,
) -> Result<(), Error> {
	let client = super::get_kubernetes_client(kubeconfig).await?;

	log::trace!("request_id: {} - creating namespace", request_id);
	let kubernetes_namespace = Namespace {
		metadata: ObjectMeta {
			name: Some(namespace_name.to_string()),
			..ObjectMeta::default()
		},
		..Namespace::default()
	};
	let namespace_api = Api::<Namespace>::all(client);
	namespace_api
		.create(&PostParams::default(), &kubernetes_namespace)
		.await?;
	log::trace!("request_id: {} - namespace created", request_id);

	Ok(())
}

pub async fn delete_kubernetes_namespace(
	namespace_name: &str,
	kubeconfig: KubernetesConfig,
	request_id: &Uuid,
) -> Result<(), Error> {
	let client = super::get_kubernetes_client(kubeconfig).await?;

	log::trace!("request_id: {} - deleting namespace", request_id);

	// deleting the kubernetes namespace deletes
	// all the resources automatically that are present inside that namespace
	let namespace_api = Api::<Namespace>::all(client);
	namespace_api
		.delete_opt(
			namespace_name,
			&DeleteParams {
				propagation_policy: Some(PropagationPolicy::Foreground),
				..Default::default()
			},
		)
		.await?;
	log::trace!("request_id: {} - deleted namespace", request_id);

	Ok(())
}
