mod ci;
mod deployment;
mod workspace;

pub mod ext_traits;

use k8s_openapi::api::core::v1::Service;
use kube::{
	config::{
		AuthInfo,
		Cluster,
		Context,
		Kubeconfig,
		NamedAuthInfo,
		NamedCluster,
		NamedContext,
	},
	Api,
	Config,
};
use url::Host;

pub use self::{ci::*, deployment::*, workspace::*};
use crate::utils::{settings::Settings, Error};

async fn get_kubernetes_client(
	kube_config: Kubeconfig,
) -> Result<kube::Client, Error> {
	let kubeconfig =
		Config::from_custom_kubeconfig(kube_config, &Default::default())
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
				cluster: Some(Cluster {
					server: Some(config.kubernetes.cluster_url.clone()),
					insecure_skip_tls_verify: None,
					certificate_authority: None,
					certificate_authority_data: Some(
						config.kubernetes.certificate_authority_data.clone(),
					),
					proxy_url: None,
					extensions: None,
					..Default::default()
				}),
			}],
			auth_infos: vec![NamedAuthInfo {
				name: config.kubernetes.auth_name.clone(),
				auth_info: Some(AuthInfo {
					username: Some(config.kubernetes.auth_username.clone()),
					token: Some(config.kubernetes.auth_token.clone().into()),
					..Default::default()
				}),
			}],
			contexts: vec![NamedContext {
				name: config.kubernetes.context_name.clone(),
				context: Some(Context {
					cluster: config.kubernetes.cluster_name.clone(),
					user: config.kubernetes.auth_username.clone(),
					extensions: None,
					namespace: None,
				}),
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

pub async fn get_load_balancer_hostname(
	namespace: &str,
	service_name: &str,
	kube_config: Kubeconfig,
) -> Result<Option<Host>, Error> {
	let kube_client = get_kubernetes_client(kube_config).await?;

	let service = Api::<Service>::namespaced(kube_client, namespace)
		.get_status(service_name)
		.await?;

	let hostname = service
		.status
		.and_then(|status| status.load_balancer)
		.and_then(|load_balancer| load_balancer.ingress)
		.and_then(|load_balancer_ingresses| {
			load_balancer_ingresses.into_iter().next()
		})
		.and_then(|load_balancer_ingress| load_balancer_ingress.ip)
		.map(|hostname| {
			Host::parse(&hostname).map_err(|err| {
				log::error!(
					"Error while parsing host `{}` - `{}`",
					hostname,
					err
				);
				Error::empty().body("Hostname Parse error")
			})
		})
		.transpose()?;

	Ok(hostname)
}
