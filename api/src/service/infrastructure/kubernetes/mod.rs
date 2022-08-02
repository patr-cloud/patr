mod certificate;
mod deployment;
mod managed_url;
mod static_site;
mod workspace;

use api_models::utils::Uuid;
use k8s_openapi::api::{
	apps::v1::Deployment as K8sDeployment,
	autoscaling::v1::HorizontalPodAutoscaler,
	core::v1::{ConfigMap, Secret, Service},
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
	core::{ApiResource, DynamicObject, ErrorResponse},
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

async fn service_exists(
	service_id: &Uuid,
	kubernetes_client: kube::Client,
	namespace: &str,
) -> Result<bool, KubeError> {
	let service = Api::<Service>::namespaced(kubernetes_client, namespace)
		.get(&format!("service-{}", service_id))
		.await;
	match service {
		Err(KubeError::Api(ErrorResponse { code: 404, .. })) => Ok(false),
		Err(err) => Err(err),
		Ok(_) => Ok(true),
	}
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
	match deployment_app {
		Err(KubeError::Api(ErrorResponse { code: 404, .. })) => Ok(false),
		Err(err) => Err(err),
		Ok(_) => Ok(true),
	}
}

async fn config_mounts_map_exists(
	deployment_id: &Uuid,
	kubernetes_client: kube::Client,
	namespace: &str,
) -> Result<bool, KubeError> {
	let deployment_app =
		Api::<ConfigMap>::namespaced(kubernetes_client, namespace)
			.get(&format!("config-mount-{}", deployment_id))
			.await;
	match deployment_app {
		Err(KubeError::Api(ErrorResponse { code: 404, .. })) => Ok(false),
		Err(err) => Err(err),
		Ok(_) => Ok(true),
	}
}

async fn hpa_exists(
	hpa_id: &Uuid,
	kubernetes_client: kube::Client,
	namespace: &str,
) -> Result<bool, KubeError> {
	let hpa = Api::<HorizontalPodAutoscaler>::namespaced(
		kubernetes_client,
		namespace,
	)
	.get(&format!("hpa-{}", hpa_id))
	.await;
	match hpa {
		Err(KubeError::Api(ErrorResponse { code: 404, .. })) => Ok(false),
		Err(err) => Err(err),
		Ok(_) => Ok(true),
	}
}

async fn ingress_exists(
	managed_url_id: &Uuid,
	kubernetes_client: kube::Client,
	namespace: &str,
) -> Result<bool, KubeError> {
	let ingress = Api::<Ingress>::namespaced(kubernetes_client, namespace)
		.get(&format!("ingress-{}", managed_url_id))
		.await;
	match ingress {
		Err(KubeError::Api(ErrorResponse { code: 404, .. })) => Ok(false),
		Err(err) => Err(err),
		Ok(_) => Ok(true),
	}
}

async fn certificate_exists(
	certificate_name: &str,
	kubernetes_client: kube::Client,
	namespace: &str,
) -> Result<bool, KubeError> {
	let certificate_resource = ApiResource {
		group: "cert-manager.io".to_string(),
		version: "v1".to_string(),
		api_version: "cert-manager.io/v1".to_string(),
		kind: "certificate".to_string(),
		plural: "certificates".to_string(),
	};

	let certificate = Api::<DynamicObject>::namespaced_with(
		kubernetes_client,
		namespace,
		&certificate_resource,
	)
	.get(certificate_name)
	.await;
	match certificate {
		Err(KubeError::Api(ErrorResponse { code: 404, .. })) => Ok(false),
		Err(err) => Err(err),
		Ok(_) => Ok(true),
	}
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
