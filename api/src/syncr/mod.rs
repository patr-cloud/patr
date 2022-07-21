use api_models::utils::Uuid;

use crate::{
	db,
	service,
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

	let running_deployments =
		db::get_running_deployment_ids_for_workspace(connection, workspace_id)
			.await?;

	for deployment_id in &running_deployments {
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
		log::trace!(
			"request_id: {request_id} - Sync: deployment {deployment_id} - syncing k8s deployment"
		);
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
		log::trace!("request_id: {request_id} - Sync: deployment {deployment_id} - syncing k8s service");
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
		log::trace!("request_id: {request_id} - Sync: deployment {deployment_id} - syncing k8s hpa");
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
		log::trace!("request_id: {request_id} - Sync: deployment {deployment_id} - syncing k8s ingress");
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

	log::trace!("request_id: {request_id} - Successfully synced deployments for workspace {workspace_id} from db to k8s");

	Ok(())
}
