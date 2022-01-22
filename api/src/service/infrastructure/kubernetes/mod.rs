mod certificate;
mod deployment;
mod managed_url;
mod static_site;
mod workspace;

use api_models::utils::Uuid;
use k8s_openapi::api::{
	apps::v1::Deployment as K8sDeployment,
	core::v1::Service,
	networking::v1::Ingress,
};
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
	Error as KubeError,
};

pub use self::{
	certificate::*,
	deployment::*,
	managed_url::*,
	static_site::*,
	workspace::*,
};
use crate::utils::{settings::Settings, Error};

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
					token: Some(config.kubernetes.auth_token.clone()),
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

async fn service_exists(
	service_id: &Uuid,
	kubernetes_client: kube::Client,
	namespace: &str,
) -> Result<bool, KubeError> {
	let service = Api::<Service>::namespaced(kubernetes_client, namespace)
		.get(&format!("service-{}", service_id))
		.await;
	if let Err(KubeError::Api(error)) = service {
		if error.code == 404 {
			return Ok(false);
		} else {
			return Err(KubeError::Api(error));
		}
	}

	Ok(true)
}

async fn deployment_exists(
	deployment_id: &Uuid,
	kubernetes_client: kube::Client,
	namespace: &str,
) -> Result<bool, KubeError> {
	let deployment_app =
		Api::<K8sDeployment>::namespaced(kubernetes_client, namespace)
			.get(&format!("deployment-{}", deployment_id))
			.await;

	if let Err(KubeError::Api(error)) = deployment_app {
		if error.code == 404 {
			return Ok(false);
		} else {
			return Err(KubeError::Api(error));
		}
	}

	Ok(true)
}

async fn ingress_exists(
	managed_url_id: &Uuid,
	kubernetes_client: kube::Client,
	namespace: &str,
) -> Result<bool, KubeError> {
	let ingress = Api::<Ingress>::namespaced(kubernetes_client, namespace)
		.get(&format!("ingress-{}", managed_url_id))
		.await;
	if let Err(KubeError::Api(error)) = ingress {
		if error.code == 404 {
			return Ok(false);
		} else {
			return Err(KubeError::Api(error));
		}
	}

	Ok(true)
}
