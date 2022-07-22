use std::collections::HashSet;

use api_models::utils::Uuid;
use k8s_openapi::api::{
	apps::v1::Deployment,
	autoscaling::v1::HorizontalPodAutoscaler,
	core::v1::Service,
	networking::v1::Ingress,
};
use kube::{
	api::{DeleteParams, PropagationPolicy},
	Api,
};

use crate::{
	db,
	service::{self, ext_traits::DeleteOpt},
	utils::{settings::Settings, Error},
	Database,
};

pub async fn sync_deployments_in_workspace(
	workspace_id: &Uuid,
	connection: &mut <Database as sqlx::Database>::Connection,
	kube_client: kube::Client,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!("request_id: {request_id} - Syncing the deployments for workspace {workspace_id} from db to k8s");
	let namespace = workspace_id.as_str();

	log::trace!(
		"request_id: {request_id} - Checking to create missing resources"
	);
	let running_deployments =
		db::get_running_deployment_ids_for_workspace(connection, workspace_id)
			.await?;
	for deployment_id in &running_deployments {
		log::trace!(
			"request_id: {request_id} - Syncing deployment {deployment_id} "
		);

		let (
			deployment,
			_workspace_id,
			_full_image,
			deployment_running_details,
		) = service::get_full_deployment_config(
			&mut *connection,
			deployment_id,
			request_id,
		)
		.await?;

		// check deployment status
		if !service::deployment_exists(
			deployment_id,
			kube_client.clone(),
			namespace,
		)
		.await?
		{
			log::error!(
				"request_id: {request_id} - Sync: deployment {deployment_id} - k8s deployment missing"
			);

			let (image_name, digest) =
				service::get_image_name_and_digest_for_deployment_image(
					connection,
					&deployment.registry,
					&deployment.image_tag,
					config,
					request_id,
				)
				.await?;

			service::update_k8s_deployment_for_deployment(
				workspace_id,
				&deployment,
				&deployment_running_details,
				&image_name,
				digest.as_deref(),
				request_id,
				&kube_client,
				config,
			)
			.await?;
		}

		// check service status
		if !service::service_exists(
			deployment_id,
			kube_client.clone(),
			namespace,
		)
		.await?
		{
			log::error!(
				"request_id: {request_id} - Sync: deployment {deployment_id} - k8s servcie missing"
			);
			service::update_k8s_service_for_deployment(
				workspace_id,
				&deployment,
				&deployment_running_details,
				request_id,
				&kube_client,
			)
			.await?;
		}

		// check pod scaler status
		if !service::hpa_exists(deployment_id, kube_client.clone(), namespace)
			.await?
		{
			log::error!("request_id: {request_id} - Sync: deployment {deployment_id} - k8s hpa missing");
			service::update_k8s_hpa_for_deployment(
				workspace_id,
				&deployment,
				&deployment_running_details,
				request_id,
				&kube_client,
			)
			.await?;
		}

		// check ingress status
		if !service::ingress_exists(
			deployment_id,
			kube_client.clone(),
			namespace,
		)
		.await?
		{
			log::error!(
				"request_id: {request_id} - Sync: deployment {deployment_id} - k8s ingress missing"
			);
			service::update_k8s_ingress_for_deployment(
				workspace_id,
				&deployment,
				&deployment_running_details,
				request_id,
				&kube_client,
				config,
			)
			.await?;
		}
	}

	log::trace!(
		"request_id: {request_id} - Checking to create missing resources"
	);
	let all_deployments = db::get_all_deployment_ids_for_workspace(
		&mut *connection,
		workspace_id,
	)
	.await?
	.into_iter()
	.collect::<HashSet<_>>();

	// TODO: make sure deployments, services, hpa & ingress
	// are used only for deployments alone and not by other
	// resources like static sites

	// delete excess k8s deployments
	let deployment_api = Api::<Deployment>::namespaced(
		kube_client.clone(),
		workspace_id.as_str(),
	);
	for deployment in deployment_api.list(&Default::default()).await? {
		// deployment name won't be none
		if let Some(deployment_name) = deployment.metadata.name {
			let is_known = deployment_name
				.strip_prefix("deployment-")
				.and_then(|id_str| Uuid::parse_str(id_str).ok())
				.map_or(false, |id| all_deployments.contains(&id));

			if !is_known {
				log::info!("request_id: {request_id} - Deleting unknown deployment `{deployment_name}` from k8s");
				deployment_api
					.delete_opt(
						&deployment_name,
						&DeleteParams {
							propagation_policy: Some(
								PropagationPolicy::Foreground,
							),
							..Default::default()
						},
					)
					.await?;
			}
		}
	}

	// delete excess k8s services
	let service_api =
		Api::<Service>::namespaced(kube_client.clone(), workspace_id.as_str());
	for service in service_api.list(&Default::default()).await? {
		// service name won't be none
		if let Some(service_name) = service.metadata.name {
			let is_known = service_name
				.strip_prefix("service-")
				.and_then(|id_str| Uuid::parse_str(id_str).ok())
				.map_or(false, |id| all_deployments.contains(&id));

			if !is_known {
				log::info!("request_id: {request_id} - Deleting unknown service `{service_name}` from k8s");
				service_api
					.delete_opt(
						&service_name,
						&DeleteParams {
							propagation_policy: Some(
								PropagationPolicy::Foreground,
							),
							..Default::default()
						},
					)
					.await?;
			}
		}
	}

	// delete excess k8s hpa
	let hpa_api = Api::<HorizontalPodAutoscaler>::namespaced(
		kube_client.clone(),
		workspace_id.as_str(),
	);
	for hpa in hpa_api.list(&Default::default()).await? {
		// hpa name won't be none
		if let Some(hpa_name) = hpa.metadata.name {
			let is_known = hpa_name
				.strip_prefix("hpa-")
				.and_then(|id_str| Uuid::parse_str(id_str).ok())
				.map_or(false, |id| all_deployments.contains(&id));

			if !is_known {
				log::info!("request_id: {request_id} - Deleting unknown hpa `{hpa_name}` from k8s");
				hpa_api
					.delete_opt(
						&hpa_name,
						&DeleteParams {
							propagation_policy: Some(
								PropagationPolicy::Foreground,
							),
							..Default::default()
						},
					)
					.await?;
			}
		}
	}

	// delete excess k8s ingress
	let ingress_api =
		Api::<Ingress>::namespaced(kube_client.clone(), workspace_id.as_str());
	for ingress in ingress_api.list(&Default::default()).await? {
		// ingress name won't be none
		if let Some(ingress_name) = ingress.metadata.name {
			let is_known = ingress_name
				.strip_prefix("ingress-")
				.and_then(|id_str| Uuid::parse_str(id_str).ok())
				.map_or(false, |id| all_deployments.contains(&id));

			if !is_known {
				log::info!("request_id: {request_id} - Deleting unknown ingress `{ingress_name}` from k8s");
				ingress_api
					.delete_opt(
						&ingress_name,
						&DeleteParams {
							propagation_policy: Some(
								PropagationPolicy::Foreground,
							),
							..Default::default()
						},
					)
					.await?;
			}
		}
	}

	log::trace!("request_id: {request_id} - Successfully synced deployments for workspace {workspace_id} from db to k8s");

	Ok(())
}
