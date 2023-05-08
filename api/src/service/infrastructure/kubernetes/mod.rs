mod ci;
mod database;
mod deployment;
mod workspace;

pub mod ext_traits;

use k8s_openapi::api::core::v1::Service;
use kube::{config::Kubeconfig, Api, Config};
use url::Host;

pub use self::{ci::*, database::*, deployment::*, workspace::*};
use crate::utils::Error;

async fn get_kubernetes_client(
	kube_config: Kubeconfig,
) -> Result<kube::Client, Error> {
	let kubeconfig =
		Config::from_custom_kubeconfig(kube_config, &Default::default())
			.await?;

	let kube_client = kube::Client::try_from(kubeconfig)?;
	Ok(kube_client)
}

pub async fn get_patr_ingress_load_balancer_hostname(
	kube_config: Kubeconfig,
) -> Result<Option<Host>, Error> {
	let namespace = "ingress-nginx";
	let service_name = "ingress-nginx-controller";

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
		.and_then(|load_balancer_ingress| {
			load_balancer_ingress.ip.or(load_balancer_ingress.hostname)
		})
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
