use std::fmt::Debug;

use api_models::utils::Uuid;
use either::Either;
use futures::{stream, StreamExt, TryStreamExt};
use k8s_openapi::api::{
	core::v1::{Endpoints, Service},
	networking::v1::Ingress,
};
use kube::{
	api::DeleteParams,
	client::Status,
	config::{
		AuthInfo,
		Cluster,
		Context,
		Kubeconfig,
		NamedAuthInfo,
		NamedCluster,
		NamedContext,
	},
	error::ErrorResponse,
	Api,
	Config,
	Error as KubeError,
};
use serde::de::DeserializeOwned;
use sqlx::Row;

use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

#[derive(Debug, Clone)]
pub struct KubernetesAuthDetails {
	pub cluster_url: String,
	pub auth_username: String,
	pub auth_token: String,
	pub certificate_authority_data: String,
}

#[async_trait::async_trait]
pub trait DeleteOpt<T> {
	async fn delete_opt(
		&self,
		name: &str,
		dp: &DeleteParams,
	) -> kube::Result<Option<Either<T, Status>>>;
}

#[async_trait::async_trait]
impl<T> DeleteOpt<T> for Api<T>
where
	T: Clone + DeserializeOwned + Debug,
{
	async fn delete_opt(
		&self,
		name: &str,
		dp: &DeleteParams,
	) -> kube::Result<Option<Either<T, Status>>> {
		match self.delete(name, dp).await {
			Ok(obj) => Ok(Some(obj)),
			Err(KubeError::Api(ErrorResponse { code: 404, .. })) => Ok(None),
			Err(err) => Err(err),
		}
	}
}

fn get_kubeconfig_for_default_region(
	config: &Settings,
) -> KubernetesAuthDetails {
	KubernetesAuthDetails {
		cluster_url: config.kubernetes.cluster_url.to_owned(),
		auth_username: config.kubernetes.auth_username.to_owned(),
		auth_token: config.kubernetes.auth_token.to_owned(),
		certificate_authority_data: config
			.kubernetes
			.certificate_authority_data
			.to_owned(),
	}
}

async fn get_kubernetes_client(
	kube_auth_details: KubernetesAuthDetails,
) -> Result<kube::Client, Error> {
	let kubeconfig = Config::from_custom_kubeconfig(
		Kubeconfig {
			api_version: Some("v1".to_string()),
			kind: Some("Config".to_string()),
			clusters: vec![NamedCluster {
				name: "kubernetesCluster".to_owned(),
				cluster: Cluster {
					server: kube_auth_details.cluster_url,
					certificate_authority_data: Some(
						kube_auth_details.certificate_authority_data,
					),
					insecure_skip_tls_verify: None,
					certificate_authority: None,
					proxy_url: None,
					extensions: None,
				},
			}],
			auth_infos: vec![NamedAuthInfo {
				name: kube_auth_details.auth_username.clone(),
				auth_info: AuthInfo {
					token: Some(kube_auth_details.auth_token.into()),
					..Default::default()
				},
			}],
			contexts: vec![NamedContext {
				name: "kubernetesContext".to_owned(),
				context: Context {
					cluster: "kubernetesCluster".to_owned(),
					user: kube_auth_details.auth_username,
					extensions: None,
					namespace: None,
				},
			}],
			current_context: Some("kubernetesContext".to_owned()),
			preferences: None,
			extensions: None,
		},
		&Default::default(),
	)
	.await?;

	let kube_client = kube::Client::try_from(kubeconfig)?;
	Ok(kube_client)
}

pub async fn delete_k8s_static_site_resources(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	let static_site_details = query!(
		r#"
		SELECT
            workspace_id,
			id
		FROM static_site;
		"#
	)
	.fetch(connection)
	.map_ok(|row| {
		(row.get::<Uuid, _>("workspace_id"), row.get::<Uuid, _>("id"))
	})
	.try_collect::<Vec<_>>()
	.await?;

	let kube_client =
		get_kubernetes_client(get_kubeconfig_for_default_region(config))
			.await?;

	let total_count = static_site_details.len();
	stream::iter(static_site_details)
		.enumerate()
		.map(|(idx, (workspace_id, static_site_id))| {
			let kube_client = kube_client.clone();

			async move {
				let namespace = workspace_id.as_str();

				let svc_deletion_result =
					Api::<Service>::namespaced(kube_client.clone(), namespace)
						.delete_opt(
							&format!("service-{}", static_site_id),
							&DeleteParams::default(),
						)
						.await;
				if let Err(err) = svc_deletion_result {
					return Err((static_site_id, err));
				};

				let ingress_deletion_result =
					Api::<Ingress>::namespaced(kube_client, namespace)
						.delete_opt(
							&format!("ingress-{}", static_site_id),
							&DeleteParams::default(),
						)
						.await;
				if let Err(err) = ingress_deletion_result {
					return Err((static_site_id, err));
				}

				log::info!(
					"{}/{} - Successfully deleted k8s resources for static site {}",
					idx,
					total_count,
					static_site_id
				);

				Result::<Uuid, (Uuid, kube::Error)>::Ok(static_site_id)
			}
		})
		.buffer_unordered(num_cpus::get() * 4)
		.for_each(|task_result| async {
			match task_result {
				Ok(_) => {}
				Err((static_site_id, err)) => log::info!(
					"Error while deleting static site resource {} - {}",
					static_site_id,
					err
				),
			}
		})
		.await;

	Ok(())
}

pub async fn delete_k8s_region_resources(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	let region_details = query!(
		r#"
		SELECT
            workspace_id,
			id
		FROM deployment_region
		WHERE
			workspace_id IS NOT NULL;
		"#
	)
	.fetch(connection)
	.map_ok(|row| {
		(row.get::<Uuid, _>("workspace_id"), row.get::<Uuid, _>("id"))
	})
	.try_collect::<Vec<_>>()
	.await?;

	let kube_client =
		get_kubernetes_client(get_kubeconfig_for_default_region(config))
			.await?;

	let total_count = region_details.len();
	stream::iter(region_details)
		.enumerate()
		.map(|(idx, (workspace_id, region_id))| {
			let kube_client = kube_client.clone();

			async move {
				let namespace = workspace_id.as_str();

				let svc_deletion_result =
					Api::<Service>::namespaced(kube_client.clone(), namespace)
						.delete_opt(
							&format!("service-{}", region_id),
							&DeleteParams::default(),
						)
						.await;
				if let Err(err) = svc_deletion_result {
					return Err((region_id, err));
				};

				let endpoint_deletion_result =
					Api::<Endpoints>::namespaced(kube_client, namespace)
						.delete_opt(
							&format!("service-{}", region_id),
							&DeleteParams::default(),
						)
						.await;
				if let Err(err) = endpoint_deletion_result {
					return Err((region_id, err));
				}

				log::info!(
					"{}/{} - Successfully deleted k8s resources for region {}",
					idx,
					total_count,
					region_id
				);

				Result::<Uuid, (Uuid, kube::Error)>::Ok(region_id)
			}
		})
		.buffer_unordered(num_cpus::get() * 4)
		.for_each(|task_result| async {
			match task_result {
				Ok(_) => {}
				Err((static_site_id, err)) => log::info!(
					"Error while deleting region resource {} - {}",
					static_site_id,
					err
				),
			}
		})
		.await;

	Ok(())
}

pub async fn delete_k8s_managed_url_resources(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	let managed_url_details = query!(
		r#"
		SELECT
            workspace_id,
			id
		FROM managed_url;
		"#
	)
	.fetch(connection)
	.map_ok(|row| {
		(row.get::<Uuid, _>("workspace_id"), row.get::<Uuid, _>("id"))
	})
	.try_collect::<Vec<_>>()
	.await?;

	let kube_client =
		get_kubernetes_client(get_kubeconfig_for_default_region(config))
			.await?;

	let total_count = managed_url_details.len();
	stream::iter(managed_url_details)
		.enumerate()
		.map(|(idx, (workspace_id, managed_url_id))| {
			let kube_client = kube_client.clone();

			async move {
				let namespace = workspace_id.as_str();

				let svc_deletion_result =
					Api::<Service>::namespaced(kube_client.clone(), namespace)
						.delete_opt(
							&format!("service-{}", managed_url_id),
							&DeleteParams::default(),
						)
						.await;
				if let Err(err) = svc_deletion_result {
					return Err((managed_url_id, err));
				};

				let ingress_deletion_result = Api::<Endpoints>::namespaced(
					kube_client.clone(),
					namespace,
				)
				.delete_opt(
					&format!("ingress-{}", managed_url_id),
					&DeleteParams::default(),
				)
				.await;
				if let Err(err) = ingress_deletion_result {
					return Err((managed_url_id, err));
				}

				let svc_verification_deletion_result =
					Api::<Service>::namespaced(kube_client.clone(), namespace)
						.delete_opt(
							&format!("service-{}-verification", managed_url_id),
							&DeleteParams::default(),
						)
						.await;
				if let Err(err) = svc_verification_deletion_result {
					return Err((managed_url_id, err));
				}

				let ingress_verification_deletion_result =
					Api::<Ingress>::namespaced(kube_client, namespace)
						.delete_opt(
							&format!("ingress-{}-verification", managed_url_id),
							&DeleteParams::default(),
						)
						.await;
				if let Err(err) = ingress_verification_deletion_result {
					return Err((managed_url_id, err));
				}

				log::info!(
					"{}/{} - Successfully deleted k8s resources for region {}",
					idx,
					total_count,
					managed_url_id
				);

				Result::<Uuid, (Uuid, kube::Error)>::Ok(managed_url_id)
			}
		})
		.buffer_unordered(num_cpus::get() * 4)
		.for_each(|task_result| async {
			match task_result {
				Ok(_) => {}
				Err((static_site_id, err)) => log::info!(
					"Error while deleting region resource {} - {}",
					static_site_id,
					err
				),
			}
		})
		.await;

	Ok(())
}
