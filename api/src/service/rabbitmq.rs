use api_models::{
	models::workspace::infrastructure::deployment::{
		Deployment,
		DeploymentRegistry,
		DeploymentRunningDetails,
		DeploymentStatus,
	},
	utils::Uuid,
};
use chrono::{DateTime, Utc};
use lapin::{options::BasicPublishOptions, BasicProperties};

use crate::{
	db::{self, Workspace},
	models::{
		rabbitmq::{
			CIData,
			DeploymentRequestData,
			Queue,
			WorkspaceRequestData,
		},
		DeploymentMetadata,
	},
	rabbitmq::{self, BuildId, BuildStep},
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

	send_message_to_infra_queue(
		&DeploymentRequestData::Create {
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
		},
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

	send_message_to_infra_queue(
		&DeploymentRequestData::Start {
			workspace_id: workspace_id.clone(),
			deployment: deployment.clone(),
			image_name,
			digest,
			running_details: deployment_running_details.clone(),
			user_id: user_id.clone(),
			login_id: login_id.clone(),
			ip_address: ip_address.to_string(),
			request_id: request_id.clone(),
		},
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

	send_message_to_infra_queue(
		&DeploymentRequestData::Update {
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
		},
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

	send_message_to_infra_queue(
		&DeploymentRequestData::UpdateImage {
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
		},
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
		&WorkspaceRequestData::ProcessWorkspaces {
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
	process_after: &DateTime<Utc>,
	total_bill: f64,
	amount_due: f64,
	month: u32,
	year: i32,
	config: &Settings,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	send_message_to_billing_queue(
		&WorkspaceRequestData::AttemptToChargeWorkspace {
			workspace: workspace.clone(),
			process_after: (*process_after).into(),
			total_bill,
			amount_due,
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
		&WorkspaceRequestData::GenerateInvoice {
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
	message: &DeploymentRequestData,
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
	message: &WorkspaceRequestData,
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

pub async fn queue_create_ci_build_step(
	build_step: BuildStep,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_ci_queue(
		&CIData::BuildStep {
			build_step,
			request_id: request_id.clone(),
		},
		config,
		request_id,
	)
	.await
}

pub async fn queue_cancel_ci_build_pipeline(
	build_id: BuildId,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_ci_queue(
		&CIData::CancelBuild {
			build_id,
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
