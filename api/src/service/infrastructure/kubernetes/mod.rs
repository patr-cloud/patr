mod ci;
mod deployment;
mod managed_url;
mod static_site;
mod workspace;

pub mod ext_traits;

use std::{net::IpAddr, str::FromStr};

use api_models::utils::Uuid;
use k8s_openapi::api::core::v1::{
	EndpointAddress,
	EndpointPort,
	EndpointSubset,
	Endpoints,
	Secret,
	Service,
	ServicePort,
	ServiceSpec,
};
use kube::{
	api::{Patch, PatchParams},
	config::{
		AuthInfo,
		Cluster,
		Context,
		Kubeconfig,
		NamedAuthInfo,
		NamedCluster,
		NamedContext,
	},
	core::{ErrorResponse, ObjectMeta},
	Api,
	Config,
	Error as KubeError,
};

pub use self::{
	ci::*,
	deployment::*,
	managed_url::*,
	static_site::*,
	workspace::*,
};
use crate::{
	service::KubernetesAuthDetails,
	utils::{settings::Settings, Error},
};

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

async fn get_kubernetes_config(
	config: &Settings,
) -> Result<kube::Client, Error> {
	let config = Config::from_custom_kubeconfig(
		Kubeconfig {
			preferences: None,
			clusters: vec![NamedCluster {
				name: config.kubernetes.cluster_name.clone(),
				cluster: Cluster {
					server: config.kubernetes.cluster_url.clone(),
					insecure_skip_tls_verify: None,
					certificate_authority: None,
					certificate_authority_data: Some(
						config.kubernetes.certificate_authority_data.clone(),
					),
					proxy_url: None,
					extensions: None,
				},
			}],
			auth_infos: vec![NamedAuthInfo {
				name: config.kubernetes.auth_name.clone(),
				auth_info: AuthInfo {
					username: Some(config.kubernetes.auth_username.clone()),
					token: Some(config.kubernetes.auth_token.clone().into()),
					..Default::default()
				},
			}],
			contexts: vec![NamedContext {
				name: config.kubernetes.context_name.clone(),
				context: Context {
					cluster: config.kubernetes.cluster_name.clone(),
					user: config.kubernetes.auth_username.clone(),
					extensions: None,
					namespace: None,
				},
			}],
			current_context: Some(config.kubernetes.context_name.clone()),
			extensions: None,
			kind: Some("Config".to_string()),
			api_version: Some("v1".to_string()),
		},
		&Default::default(),
	)
	.await?;

	let client = kube::Client::try_from(config)?;

	Ok(client)
}

async fn secret_exists(
	secret_name: &str,
	kubernetes_client: kube::Client,
	namespace: &str,
) -> Result<bool, KubeError> {
	let secret = Api::<Secret>::namespaced(kubernetes_client, namespace)
		.get(secret_name)
		.await;
	match secret {
		Err(KubeError::Api(ErrorResponse { code: 404, .. })) => Ok(false),
		Err(err) => Err(err),
		Ok(_) => Ok(true),
	}
}

pub async fn get_external_ip_addr_for_load_balancer(
	namespace: &str,
	service_name: &str,
	kube_auth_details: KubernetesAuthDetails,
) -> Result<Option<IpAddr>, Error> {
	let kube_client = get_kubernetes_client(kube_auth_details).await?;

	let service = Api::<Service>::namespaced(kube_client, namespace)
		.get_status(service_name)
		.await?;

	let ip_addr = service
		.status
		.and_then(|status| status.load_balancer)
		.and_then(|load_balancer| load_balancer.ingress)
		.and_then(|load_balancer_ingresses| {
			load_balancer_ingresses.into_iter().next()
		})
		.and_then(|load_balancer_ingress| load_balancer_ingress.ip)
		.and_then(|ip_addr| IpAddr::from_str(&ip_addr).ok());

	Ok(ip_addr)
}

pub async fn create_external_service_for_region(
	parent_namespace: &str,
	region_id: &Uuid,
	external_ip_addr: &IpAddr,
	kube_auth_details: KubernetesAuthDetails,
) -> Result<(), Error> {
	let kube_client = get_kubernetes_client(kube_auth_details).await?;

	Api::<Service>::namespaced(kube_client.clone(), parent_namespace)
		.patch(
			&format!("service-{}", region_id),
			&PatchParams::apply(&format!("service-{}", region_id)),
			&Patch::Apply(Service {
				metadata: ObjectMeta {
					name: Some(format!("service-{}", region_id)),
					..ObjectMeta::default()
				},
				spec: Some(ServiceSpec {
					ports: Some(vec![ServicePort {
						port: 80,
						protocol: Some("TCP".to_owned()),
						..Default::default()
					}]),
					..ServiceSpec::default()
				}),
				..Service::default()
			}),
		)
		.await?;

	Api::<Endpoints>::namespaced(kube_client, parent_namespace)
		.patch(
			&format!("service-{}", region_id),
			&PatchParams::apply(&format!("service-{}", region_id)),
			&Patch::Apply(Endpoints {
				metadata: ObjectMeta {
					name: Some(format!("service-{}", region_id)),
					..ObjectMeta::default()
				},
				subsets: Some(vec![EndpointSubset {
					addresses: Some(vec![EndpointAddress {
						ip: external_ip_addr.to_string(),
						..Default::default()
					}]),
					ports: Some(vec![EndpointPort {
						port: 80,
						..Default::default()
					}]),
					..Default::default()
				}]),
			}),
		)
		.await?;

	Ok(())
}
