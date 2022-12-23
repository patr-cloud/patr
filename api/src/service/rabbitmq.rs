use api_models::{
	models::{
		ci::file_format::{Service, Work},
		workspace::infrastructure::deployment::{
			Deployment,
			DeploymentRegistry,
			DeploymentRunningDetails,
			DeploymentStatus,
		},
	},
	utils::Uuid,
};
use chrono::{DateTime, Utc};
use kube::config::Kubeconfig;
use lapin::{options::BasicPublishOptions, BasicProperties};

use crate::{
	db::Workspace,
	models::rabbitmq::{
		BYOCData,
		BillingData,
		CIData,
		DatabaseRequestData,
		DeploymentRequestData,
		DockerRegistryData,
		InfraRequestData,
		Queue,
	},
	rabbitmq::{self, BuildId, BuildStep},
	service,
	utils::{settings::Settings, Error},
};

pub async fn queue_check_and_update_deployment_status(
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_infra_queue(
		&InfraRequestData::Deployment(
			DeploymentRequestData::CheckAndUpdateStatus {
				workspace_id: workspace_id.clone(),
				deployment_id: deployment_id.clone(),
			},
		),
		config,
		request_id,
	)
	.await
}

pub async fn queue_update_deployment_image(
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	name: &str,
	registry: &DeploymentRegistry,
	image_name: &str,
	digest: &str,
	image_tag: &str,
	region: &Uuid,
	machine_type: &Uuid,
	deployment_running_details: &DeploymentRunningDetails,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_infra_queue(
		&InfraRequestData::Deployment(DeploymentRequestData::UpdateImage {
			workspace_id: workspace_id.clone(),
			deployment: Deployment {
				id: deployment_id.clone(),
				name: name.to_string(),
				registry: registry.clone(),
				image_tag: image_tag.to_string(),
				status: DeploymentStatus::Pushed,
				region: region.clone(),
				machine_type: machine_type.clone(),
				current_live_digest: Some(digest.to_string()),
			},
			image_name: image_name.to_owned(),
			digest: digest.to_string(),
			running_details: deployment_running_details.clone(),
			request_id: request_id.clone(),
		}),
		config,
		request_id,
	)
	.await
}

pub async fn queue_check_and_update_database_status(
	workspace_id: &Uuid,
	database_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_infra_queue(
		&InfraRequestData::Database(
			DatabaseRequestData::CheckAndUpdateStatus {
				workspace_id: workspace_id.clone(),
				database_id: database_id.clone(),
				request_id: request_id.clone(),
			},
		),
		config,
		request_id,
	)
	.await
}

pub async fn queue_delete_docker_registry_image(
	workspace_id: &Uuid,
	repository_name: &str,
	digest: &str,
	tag: &str,
	image_pushed_ip_addr: &str,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_infra_queue(
		&InfraRequestData::DockerRegistry(
			DockerRegistryData::DeleteDockerImage {
				request_id: request_id.clone(),
				workspace_id: workspace_id.clone(),
				repository_name: repository_name.to_owned(),
				digest: digest.to_owned(),
				tag: tag.to_owned(),
				image_pushed_ip_addr: image_pushed_ip_addr.to_owned(),
			},
		),
		config,
		request_id,
	)
	.await
}

pub async fn queue_process_payment(
	month: u32,
	year: i32,
	config: &Settings,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	send_message_to_billing_queue(
		&BillingData::ProcessWorkspaces {
			month,
			year,
			request_id: request_id.clone(),
		},
		config,
		&request_id,
	)
	.await
}

pub async fn queue_attempt_to_charge_workspace(
	workspace: &Workspace,
	month: u32,
	year: i32,
	config: &Settings,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	send_message_to_billing_queue(
		&BillingData::SendInvoiceForWorkspace {
			workspace: workspace.clone(),
			month,
			year,
			request_id: request_id.clone(),
		},
		config,
		&request_id,
	)
	.await
}

pub async fn queue_retry_payment_for_workspace(
	workspace_id: &Uuid,
	process_after: &DateTime<Utc>,
	month: u32,
	year: i32,
	config: &Settings,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	send_message_to_billing_queue(
		&BillingData::RetryPaymentForWorkspace {
			workspace_id: workspace_id.clone(),
			process_after: (*process_after).into(),
			month,
			year,
			request_id: request_id.clone(),
		},
		config,
		&request_id,
	)
	.await
}

pub async fn queue_generate_invoice_for_workspace(
	config: &Settings,
	workspace: Workspace,
	month: u32,
	year: i32,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();

	send_message_to_billing_queue(
		&BillingData::GenerateInvoice {
			month,
			year,
			workspace,
			request_id: request_id.clone(),
		},
		config,
		&request_id,
	)
	.await
}

pub async fn send_message_to_infra_queue(
	message: &InfraRequestData,
	_config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let app = service::get_app();
	let (channel, rabbitmq_connection) =
		rabbitmq::get_rabbitmq_connection_channel(&app.rabbitmq).await?;

	let confirmation = channel
		.basic_publish(
			"",
			&Queue::Infrastructure.to_string(),
			BasicPublishOptions::default(),
			serde_json::to_string(&message)?.as_bytes(),
			BasicProperties::default(),
		)
		.await?
		.await?;

	if !confirmation.is_ack() {
		log::error!("request_id: {} - RabbitMQ publish failed", request_id);
		return Err(Error::empty());
	}

	channel.close(200, "Normal shutdown").await.map_err(|e| {
		log::error!("Error closing rabbitmq channel: {}", e);
		Error::from(e)
	})?;

	rabbitmq_connection
		.close(200, "Normal shutdown")
		.await
		.map_err(|e| {
			log::error!("Error closing rabbitmq connection: {}", e);
			Error::from(e)
		})?;
	Ok(())
}

pub async fn send_message_to_ci_queue(
	message: &CIData,
	_config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let app = service::get_app();
	let (channel, rabbitmq_connection) =
		rabbitmq::get_rabbitmq_connection_channel(&app.rabbitmq).await?;

	let confirmation = channel
		.basic_publish(
			"",
			&Queue::Ci.to_string(),
			BasicPublishOptions::default(),
			serde_json::to_string(&message)?.as_bytes(),
			BasicProperties::default(),
		)
		.await?
		.await?;

	if !confirmation.is_ack() {
		log::error!("request_id: {} - RabbitMQ publish failed", request_id);
		return Err(Error::empty());
	}

	channel.close(200, "Normal shutdown").await.map_err(|e| {
		log::error!("Error closing rabbitmq channel: {}", e);
		Error::from(e)
	})?;

	rabbitmq_connection
		.close(200, "Normal shutdown")
		.await
		.map_err(|e| {
			log::error!("Error closing rabbitmq connection: {}", e);
			Error::from(e)
		})?;
	Ok(())
}

pub async fn send_message_to_billing_queue(
	message: &BillingData,
	_config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let app = service::get_app();
	let (channel, rabbitmq_connection) =
		rabbitmq::get_rabbitmq_connection_channel(&app.rabbitmq).await?;

	let confirmation = channel
		.basic_publish(
			"",
			&Queue::Billing.to_string(),
			BasicPublishOptions::default(),
			serde_json::to_string(&message)?.as_bytes(),
			BasicProperties::default(),
		)
		.await?
		.await?;

	if !confirmation.is_ack() {
		log::error!("request_id: {} - RabbitMQ publish failed", request_id);
		return Err(Error::empty());
	}

	channel.close(200, "Normal shutdown").await.map_err(|e| {
		log::error!("Error closing rabbitmq channel: {}", e);
		Error::from(e)
	})?;

	rabbitmq_connection
		.close(200, "Normal shutdown")
		.await
		.map_err(|e| {
			log::error!("Error closing rabbitmq connection: {}", e);
			Error::from(e)
		})?;
	Ok(())
}

pub async fn queue_check_and_start_ci_build(
	build_id: BuildId,
	services: Vec<Service>,
	work_steps: Vec<Work>,
	event_type: EventType,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_ci_queue(
		&CIData::CheckAndStartBuild {
			build_id,
			services,
			work_steps,
			event_type,
			request_id: request_id.clone(),
		},
		config,
		request_id,
	)
	.await
}

pub async fn queue_create_ci_build_step(
	build_step: BuildStep,
	event_type: EventType,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_ci_queue(
		&CIData::BuildStep {
			build_step,
			event_type,
			request_id: request_id.clone(),
		},
		config,
		request_id,
	)
	.await
}

pub async fn queue_clean_ci_build_pipeline(
	build_id: BuildId,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_ci_queue(
		&CIData::CleanBuild {
			build_id,
			request_id: request_id.clone(),
		},
		config,
		request_id,
	)
	.await
}

pub async fn queue_setup_kubernetes_cluster(
	region_id: &Uuid,
	kube_config: Kubeconfig,
	tls_cer: &str,
	tls_key: &str,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_infra_queue(
		&InfraRequestData::BYOC(BYOCData::InitKubernetesCluster {
			region_id: region_id.clone(),
			kube_config,
			tls_cert: tls_cer.to_owned(),
			tls_key: tls_key.to_owned(),
			request_id: request_id.clone(),
		}),
		config,
		request_id,
	)
	.await
}

pub async fn queue_sync_github_repo(
	workspace_id: &Uuid,
	git_provider_id: &Uuid,
	request_id: &Uuid,
	github_access_token: String,
	config: &Settings,
) -> Result<(), Error> {
	send_message_to_ci_queue(
		&CIData::SyncRepo {
			workspace_id: workspace_id.clone(),
			git_provider_id: git_provider_id.clone(),
			request_id: request_id.clone(),
			github_access_token,
		},
		config,
		request_id,
	)
	.await
}

pub async fn queue_docker_notification_error(
	request_body: &str,
	content_type: &str,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_docker_webhook_queue(
		&DockerWebhookData::NotificationHandler {
			request_body: request_body.to_string(),
			content_type: content_type.to_string(),
			request_id: request_id.clone(),
		},
		request_id,
	)
	.await
}

pub async fn send_message_to_docker_webhook_queue(
	message: &DockerWebhookData,
	request_id: &Uuid,
) -> Result<(), Error> {
	let app = service::get_app();
	let (channel, rabbitmq_connection) =
		rabbitmq::get_rabbitmq_connection_channel(&app.rabbitmq).await?;

	let confirmation = channel
		.basic_publish(
			"",
			"dockerWebhookError", /* not putting this in Queue enum as we
			                       * don't want this handle/consume */
			BasicPublishOptions::default(),
			serde_json::to_string(&message)?.as_bytes(),
			BasicProperties::default(),
		)
		.await?
		.await?;

	if !confirmation.is_ack() {
		log::error!("request_id: {} - RabbitMQ publish failed", request_id);
		return Err(Error::empty());
	}

	channel.close(200, "Normal shutdown").await.map_err(|e| {
		log::error!("Error closing rabbitmq channel: {}", e);
		Error::from(e)
	})?;

	rabbitmq_connection
		.close(200, "Normal shutdown")
		.await
		.map_err(|e| {
			log::error!("Error closing rabbitmq connection: {}", e);
			Error::from(e)
		})?;
	Ok(())
}

pub async fn queue_get_kube_config_for_do_cluster(
	api_token: &str,
	cluster_id: &Uuid,
	region_id: &Uuid,
	tls_cer: &str,
	tls_key: &str,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_infra_queue(
		&InfraRequestData::BYOC(BYOCData::GetDigitalOceanKubeconfig {
			api_token: api_token.to_string(),
			cluster_id: cluster_id.clone(),
			region_id: region_id.clone(),
			tls_cert: tls_cer.to_owned(),
			tls_key: tls_key.to_owned(),
			request_id: request_id.clone(),
		}),
		config,
		request_id,
	)
	.await
}

pub async fn queue_delete_kubernetes_cluster(
	region_id: &Uuid,
	workspace_id: &Uuid,
	kube_config: Kubeconfig,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_infra_queue(
		&InfraRequestData::BYOC(BYOCData::DeleteKubernetesCluster {
			region_id: region_id.clone(),
			workspace_id: workspace_id.clone(),
			kube_config,
			request_id: request_id.clone(),
		}),
		config,
		request_id,
	)
	.await
}

pub async fn queue_create_static_site_upload(
	static_site_id: &Uuid,
	upload_id: &Uuid,
	file: String,
	files_length: usize,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_infra_queue(
		&InfraRequestData::StaticSite(StaticSiteData::CreateStaticSiteUpload {
			static_site_id: static_site_id.clone(),
			upload_id: upload_id.clone(),
			file,
			files_length,
			request_id: request_id.clone(),
		}),
		config,
		request_id,
	)
	.await
}
