use api_models::utils::Uuid;
use k8s_openapi::{self, api::core::v1::Secret};
use kube::{
	self,
	api::{Patch, PatchParams},
	config::Kubeconfig,
	core::ObjectMeta,
	Api,
};

use crate::utils::{constants::PATR_BYOC_TOKEN_NAME, Error};

pub async fn copy_patr_token_to_ci_namespace(
	repo_workspace_id: &Uuid,
	build_namespace: &str,
	kubeconfig: Kubeconfig,
	request_id: &Uuid,
) -> Result<bool, Error> {
	let client = super::get_kubernetes_client(kubeconfig).await?;

	log::trace!(
		"request_id: {} - copying patr token to ci namepsace",
		request_id
	);
	let Some(existing_token) =
		Api::<Secret>::namespaced(client.clone(), repo_workspace_id.as_str())
			.get_opt(PATR_BYOC_TOKEN_NAME)
			.await? else {
				return Ok(false)
			};

	Api::<Secret>::namespaced(client.clone(), build_namespace)
		.patch(
			PATR_BYOC_TOKEN_NAME,
			&PatchParams::apply(PATR_BYOC_TOKEN_NAME),
			&Patch::Apply(Secret {
				metadata: ObjectMeta {
					name: Some(PATR_BYOC_TOKEN_NAME.to_string()),
					namespace: Some(build_namespace.to_string()),
					..Default::default()
				},
				type_: existing_token.type_,
				data: existing_token.data,
				..Default::default()
			}),
		)
		.await?;

	log::trace!(
		"request_id: {} - patr token created for ci namespace",
		request_id
	);

	Ok(true)
}

pub async fn is_patr_token_exists(
	namespace: &str,
	kubeconfig: Kubeconfig,
	request_id: &Uuid,
) -> Result<bool, Error> {
	let client = super::get_kubernetes_client(kubeconfig).await?;

	log::trace!(
		"request_id: {} - check whether patr token to exists or not",
		request_id
	);
	let result = Api::<Secret>::namespaced(client.clone(), namespace)
		.get_opt(PATR_BYOC_TOKEN_NAME)
		.await?;

	Ok(result.is_some())
}
