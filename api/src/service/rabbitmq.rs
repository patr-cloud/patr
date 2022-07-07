use api_models::{
	models::workspace::infrastructure::deployment::{
		Deployment,
		DeploymentRegistry,
		DeploymentRunningDetails,
		DeploymentStatus,
	},
	utils::Uuid,
};
use k8s_openapi::api::batch::v1::Job;
use lapin::{options::BasicPublishOptions, BasicProperties};

use crate::{
	db::{self, Workspace},
	models::{
		rabbitmq::{
			CIData,
			DeploymentRequestData,
			RequestMessage,
			WorkspaceRequestData,
		},
		DeploymentMetadata,
	},
	rabbitmq::{self, BuildId, BuildStepId},
	service,
	utils::{settings::Settings, Error},
	Database,
};

pub async fn queue_create_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	name: &str,
	registry: &DeploymentRegistry,
	image_tag: &str,
	region: &Uuid,
	machine_type: &Uuid,
	deployment_running_details: &DeploymentRunningDetails,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	// If deploy_on_create is true, then tell the consumer to create a
	// deployment
	let (image_name, digest) =
		service::get_image_name_and_digest_for_deployment_image(
			connection, registry, image_tag, config, request_id,
		)
		.await?;

	db::update_deployment_status(
		connection,
		deployment_id,
		&DeploymentStatus::Created,
	)
	.await?;

	send_message_to_rabbit_mq(
		&RequestMessage::Deployment(DeploymentRequestData::Create {
			workspace_id: workspace_id.clone(),
			deployment: Deployment {
				id: deployment_id.clone(),
				name: name.to_string(),
				registry: registry.clone(),
				image_tag: image_tag.to_string(),
				status: DeploymentStatus::Pushed,
				region: region.clone(),
				machine_type: machine_type.clone(),
				current_live_digest: digest.clone(),
			},
			image_name,
			digest,
			running_details: deployment_running_details.clone(),
			request_id: request_id.clone(),
		}),
		config,
		request_id,
	)
	.await
}

pub async fn queue_start_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	deployment: &Deployment,
	deployment_running_details: &DeploymentRunningDetails,
	user_id: &Uuid,
	login_id: &Uuid,
	ip_address: &str,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	// If deploy_on_create is true, then tell the consumer to create a
	// deployment
	let (image_name, digest) =
		service::get_image_name_and_digest_for_deployment_image(
			connection,
			&deployment.registry,
			&deployment.image_tag,
			config,
			request_id,
		)
		.await?;

	db::update_deployment_status(
		connection,
		deployment_id,
		&DeploymentStatus::Deploying,
	)
	.await?;

	send_message_to_rabbit_mq(
		&RequestMessage::Deployment(DeploymentRequestData::Start {
			workspace_id: workspace_id.clone(),
			deployment: deployment.clone(),
			image_name,
			digest,
			running_details: deployment_running_details.clone(),
			user_id: user_id.clone(),
			login_id: login_id.clone(),
			ip_address: ip_address.to_string(),
			request_id: request_id.clone(),
		}),
		config,
		request_id,
	)
	.await
}

pub async fn queue_stop_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	user_id: &Uuid,
	login_id: &Uuid,
	ip_address: &str,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	// TODO: implement logic for handling domains of the stopped deployment
	log::trace!("request_id: {} - Updating deployment status", request_id);
	db::update_deployment_status(
		connection,
		deployment_id,
		&DeploymentStatus::Stopped,
	)
	.await?;

	send_message_to_rabbit_mq(
		&RequestMessage::Deployment(DeploymentRequestData::Stop {
			workspace_id: workspace_id.clone(),
			deployment_id: deployment_id.clone(),
			user_id: user_id.clone(),
			login_id: login_id.clone(),
			ip_address: ip_address.to_string(),
			request_id: request_id.clone(),
		}),
		config,
		request_id,
	)
	.await
}

pub async fn queue_delete_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	name: &str,
	user_id: &Uuid,
	login_id: &Uuid,
	ip_address: &str,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - Updating the deployment name in the database",
		request_id
	);
	db::update_deployment_name(
		connection,
		deployment_id,
		&format!("patr-deleted: {}-{}", name, deployment_id),
	)
	.await?;

	log::trace!("request_id: {} - Updating deployment status", request_id);
	db::update_deployment_status(
		connection,
		deployment_id,
		&DeploymentStatus::Deleted,
	)
	.await?;

	send_message_to_rabbit_mq(
		&RequestMessage::Deployment(DeploymentRequestData::Delete {
			workspace_id: workspace_id.clone(),
			deployment_id: deployment_id.clone(),
			user_id: user_id.clone(),
			login_id: login_id.clone(),
			ip_address: ip_address.to_string(),
			request_id: request_id.clone(),
		}),
		config,
		request_id,
	)
	.await
}

pub async fn queue_update_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	name: &str,
	registry: &DeploymentRegistry,
	image_tag: &str,
	region: &Uuid,
	machine_type: &Uuid,
	deployment_running_details: &DeploymentRunningDetails,
	user_id: &Uuid,
	login_id: &Uuid,
	ip_address: &str,
	metadata: &DeploymentMetadata,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let (image_name, digest) =
		service::get_image_name_and_digest_for_deployment_image(
			connection, registry, image_tag, config, request_id,
		)
		.await?;

	db::update_deployment_status(
		connection,
		deployment_id,
		&DeploymentStatus::Deploying,
	)
	.await?;

	send_message_to_rabbit_mq(
		&RequestMessage::Deployment(DeploymentRequestData::Update {
			workspace_id: workspace_id.clone(),
			deployment: Deployment {
				id: deployment_id.clone(),
				name: name.to_string(),
				registry: registry.clone(),
				image_tag: image_tag.to_string(),
				status: DeploymentStatus::Pushed,
				region: region.clone(),
				machine_type: machine_type.clone(),
				current_live_digest: digest.clone(),
			},
			image_name,
			digest,
			running_details: deployment_running_details.clone(),
			user_id: user_id.clone(),
			login_id: login_id.clone(),
			ip_address: ip_address.to_string(),
			metadata: metadata.clone(),
			request_id: request_id.clone(),
		}),
		config,
		request_id,
	)
	.await
}

pub async fn queue_update_deployment_image(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	name: &str,
	registry: &DeploymentRegistry,
	digest: &str,
	image_tag: &str,
	region: &Uuid,
	machine_type: &Uuid,
	deployment_running_details: &DeploymentRunningDetails,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let (image_name, _) =
		service::get_image_name_and_digest_for_deployment_image(
			connection, registry, image_tag, config, request_id,
		)
		.await?;

	db::update_deployment_status(
		connection,
		deployment_id,
		&DeploymentStatus::Deploying,
	)
	.await?;

	send_message_to_rabbit_mq(
		&RequestMessage::Deployment(DeploymentRequestData::UpdateImage {
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
			image_name,
			digest: Some(digest.to_string()),
			running_details: deployment_running_details.clone(),
			request_id: request_id.clone(),
		}),
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
	send_message_to_rabbit_mq(
		&RequestMessage::Workspace(WorkspaceRequestData::ProcessWorkspaces {
			month,
			year,
			request_id: request_id.clone(),
		}),
		config,
		&request_id,
	)
	.await
}

pub async fn queue_confirm_payment_intent(
	workspace_id: &Uuid,
	payment_intent_id: String,
	config: &Settings,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	send_message_to_rabbit_mq(
		&RequestMessage::Workspace(
			WorkspaceRequestData::ConfirmPaymentIntent {
				payment_intent_id,
				workspace_id: workspace_id.clone(),
				request_id: request_id.clone(),
			},
		),
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

	send_message_to_rabbit_mq(
		&RequestMessage::Workspace(WorkspaceRequestData::GenerateInvoice {
			month,
			year,
			workspace,
			request_id: request_id.clone(),
		}),
		config,
		&request_id,
	)
	.await
}

pub async fn send_message_to_rabbit_mq(
	message: &RequestMessage,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let app = service::get_app();
	let (channel, rabbitmq_connection) =
		rabbitmq::get_rabbitmq_connection_channel(&app.rabbitmq).await?;

	let confirmation = channel
		.basic_publish(
			"",
			config.rabbit_mq.queue.as_str(),
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

pub async fn queue_create_managed_url(
	workspace_id: &Uuid,
	managed_url_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_rabbit_mq(
		&RequestMessage::ManagedUrl(ManagedUrlData::Create {
			managed_url_id: managed_url_id.clone(),
			workspace_id: workspace_id.clone(),
			request_id: request_id.clone(),
		}),
		config,
		request_id,
	)
	.await?;
	Ok(())
}

pub async fn queue_clone_ci_repo(
	build_step_id: BuildStepId,
	job: Job,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_rabbit_mq(
		&RequestMessage::ContinuousIntegration(CIData::InitRepo {
			build_step_id,
			job,
			request_id: request_id.clone(),
		}),
		config,
		request_id,
	)
	.await?;
	Ok(())
}

pub async fn queue_create_build_step(
	build_step_id: BuildStepId,
	job: Job,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_rabbit_mq(
		&RequestMessage::ContinuousIntegration(CIData::CreateBuildStep {
			build_step_id,
			job,
			request_id: request_id.clone(),
		}),
		config,
		request_id,
	)
	.await?;
	Ok(())
}

pub async fn queue_update_build_step_status(
	build_step_id: BuildStepId,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_rabbit_mq(
		&RequestMessage::ContinuousIntegration(CIData::UpdateBuildStepStatus {
			build_step_id,
			request_id: request_id.clone(),
		}),
		config,
		request_id,
	)
	.await?;
	Ok(())
}

pub async fn queue_clean_build_pipeline(
	build_id: BuildId,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_rabbit_mq(
		&RequestMessage::ContinuousIntegration(CIData::CleanBuild {
			build_id,
			request_id: request_id.clone(),
		}),
		config,
		request_id,
	)
	.await?;
	Ok(())
}
