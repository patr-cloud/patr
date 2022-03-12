use api_models::utils::Uuid;
use k8s_openapi::api::core::v1::Secret;
use kube::{
	api::{DeleteParams, Patch, PatchParams},
	core::ObjectMeta,
	Api,
};

use crate::utils::{settings::Settings, Error};

pub async fn update_kuberenetes_secrets(
	workspace_id: &Uuid,
	secret_id: &Uuid,
	secret: &str,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	//TODO: connect deployment_id to secret
	let kubernetes_client = super::get_kubernetes_config(config).await?;
	let namespace = workspace_id.as_str();

	let kubernetes_secret = Secret {
		metadata: ObjectMeta {
			name: Some(format!("secret-{}", secret_id)),
			namespace: Some(namespace.to_string()),
			..ObjectMeta::default()
		},
		type_: Some("Opaque".to_string()),
		immutable: Some(true),
		string_data: Some(
			[("data".to_string(), secret.to_string())]
				.into_iter()
				.collect(),
		),
		..Secret::default()
	};

	let secret_api =
		Api::<Secret>::namespaced(kubernetes_client.clone(), namespace);

	secret_api
		.patch(
			&format!("service-{}", secret_id),
			&PatchParams::apply(&format!("secret-{}", secret_id)),
			&Patch::Apply(kubernetes_secret),
		)
		.await?;

	log::trace!("request_id: {} - secret created", request_id);

	Ok(())
}

// TODO: maybe not use this function and just fetch from database?
// pub async fn get_kubernetes_secret(
// 	workspace_id: &Uuid,
// 	secret_id: &Uuid,
// 	config: &Settings,
// 	request_id: &Uuid,
// ) -> Result<Vec<Secret>, Error> {
// 	log::trace!(
// 		"request_id: {} - request to get list of secrets for - {}",
// 		request_id,
// 		secret_id
// 	);

// 	let kubernetes_client = super::get_kubernetes_config(config).await?;
// 	let namespace = workspace_id.as_str();

// 	let secrets =
// 		Api::<Secret>::namespaced(kubernetes_client.clone(), namespace)
// 			.list(&ListParams::default())
// 			.await?
// 			.items
// 			.into_iter()
// 			.filter(|secret| match &secret.metadata.name {
// 				Some(name) => {
// 					if name.contains("secret-") {
// 						return true;
// 					} else {
// 						return false;
// 					}
// 				}
// 				None => return false,
// 			})
// 			.collect::<Vec<Secret>>();

// 	log::trace!("request_id: {} - secrets listed", request_id);

// 	// TODO: sync secrets with database
// 	// if the secret does not exist in the database, create it or you can delete
// it from database 	// let secret_ids = Vec::new();

// 	// for secret in secrets {
// 	// 	let secret_id = if let Some(secret_name) = &secret.metadata.name {
// 	// 		secret_name.replace("secret-", "")
// 	// 	} else {
// 	// 		continue;
// 	// 	};

// 	// 	secret_ids.push(secret_id);
// 	// }

// 	Ok(secrets)
// }

pub async fn delete_kubernetes_secret(
	workspace_id: &Uuid,
	secret_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - request to delete secret - {}",
		request_id,
		secret_id
	);
	let kubernetes_client = super::get_kubernetes_config(config).await?;
	let namespace = workspace_id.as_str();

	// TODO - check if the secret resource is present in DB with secret_id as
	// passed in params if exists them do the below delete operation,
	// otherwise there will be a else condition which says the secret doesn't
	// exist

	Api::<Secret>::namespaced(kubernetes_client.clone(), namespace)
		.delete(&format!("secret-{}", secret_id), &DeleteParams::default())
		.await?;

	Ok(())
}
