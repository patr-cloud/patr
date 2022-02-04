use std::collections::BTreeMap;

use api_models::utils::Uuid;
use eve_rs::AsError;
use k8s_openapi::{
	self,
	api::core::v1::{Namespace, Secret},
};
use kube::{
	self,
	api::{Patch, PatchParams, PostParams},
	core::ObjectMeta,
	Api,
};

use crate::{
	error,
	utils::{settings::Settings, Error},
};

pub async fn create_kubernetes_namespace(
	namespace_name: &str,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let client = super::get_kubernetes_config(config).await?;

	log::trace!("request_id: {} - creating namespace", request_id);
	let kubernetes_namespace = Namespace {
		metadata: ObjectMeta {
			name: Some(namespace_name.to_string()),
			..ObjectMeta::default()
		},
		..Namespace::default()
	};
	let namespace_api = Api::<Namespace>::all(client.clone());
	namespace_api
		.create(&PostParams::default(), &kubernetes_namespace)
		.await?;
	log::trace!("request_id: {} - namespace created", request_id);

	log::trace!(
		"request_id: {} - creating certificate secret mirror",
		request_id
	);

	let wild_card_secret = Api::<Secret>::namespaced(client.clone(), "default")
		.get("tls-domain-domain-wildcard-patr-cloud")
		.await?;

	let mut annotations = wild_card_secret
		.metadata
		.annotations
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?
		.into_iter()
		.filter(|(key, _)| !key.contains("cert-manager.io"))
		.collect::<BTreeMap<String, String>>();

	let mut reflector_annotation = [(
		"reflector.v1.k8s.emberstack.com/reflects".to_string(),
		"default/tls-domain-domain-wildcard-patr-cloud".to_string(),
	)]
	.into_iter()
	.collect();

	annotations.append(&mut reflector_annotation);

	let wild_card_secret = Secret {
		data: wild_card_secret.data,
		immutable: wild_card_secret.immutable,
		metadata: ObjectMeta {
			annotations: Some(annotations),
			name: Some("tls-domain-domain-wildcard-patr-cloud".to_string()),
			namespace: Some(namespace_name.to_string()),
			..ObjectMeta::default()
		},
		..Secret::default()
	};

	Api::<Secret>::namespaced(client, namespace_name)
		.patch(
			"tls-domain-domain-wildcard-patr-cloud",
			&PatchParams::apply("tls-domain-domain-wildcard-patr-cloud"),
			&Patch::Apply(wild_card_secret),
		)
		.await?;

	Ok(())
}
