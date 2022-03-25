use api_models::{
	models::workspace::infrastructure::{
		deployment::{
			Deployment,
			DeploymentRegistry,
			DeploymentRunningDetails,
			DeploymentStatus,
		},
		static_site::StaticSiteDetails,
	},
	utils::Uuid,
};
use lapin::{options::BasicPublishOptions, BasicProperties};

use crate::{
	db,
	models::{
		rabbitmq::{
			DeploymentRequestData,
			RequestMessage,
			StaticSiteRequestData,
		},
		DeploymentMetadata,
	},
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

pub async fn queue_create_static_site(
	workspace_id: &Uuid,
	static_site_id: &Uuid,
	file: String,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_rabbit_mq(
		&RequestMessage::StaticSite(StaticSiteRequestData::Create {
			workspace_id: workspace_id.clone(),
			static_site_id: static_site_id.clone(),
			file,
			static_site_details: StaticSiteDetails {},
			request_id: request_id.clone(),
		}),
		config,
		request_id,
	)
	.await
}

pub async fn queue_start_static_site(
	workspace_id: &Uuid,
	static_site_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_rabbit_mq(
		&RequestMessage::StaticSite(StaticSiteRequestData::Start {
			workspace_id: workspace_id.clone(),
			static_site_id: static_site_id.clone(),
			static_site_details: StaticSiteDetails {},
			request_id: request_id.clone(),
		}),
		config,
		request_id,
	)
	.await
}

pub async fn queue_upload_static_site(
	workspace_id: &Uuid,
	static_site_id: &Uuid,
	file: String,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_rabbit_mq(
		&RequestMessage::StaticSite(StaticSiteRequestData::UploadSite {
			workspace_id: workspace_id.clone(),
			static_site_id: static_site_id.clone(),
			file,
			request_id: request_id.clone(),
		}),
		config,
		request_id,
	)
	.await
}

pub async fn queue_stop_static_site(
	workspace_id: &Uuid,
	static_site_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_rabbit_mq(
		&RequestMessage::StaticSite(StaticSiteRequestData::Stop {
			workspace_id: workspace_id.clone(),
			static_site_id: static_site_id.clone(),
			request_id: request_id.clone(),
		}),
		config,
		request_id,
	)
	.await
}

pub async fn queue_delete_static_site(
	workspace_id: &Uuid,
	static_site_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_rabbit_mq(
		&RequestMessage::StaticSite(StaticSiteRequestData::Delete {
			workspace_id: workspace_id.clone(),
			static_site_id: static_site_id.clone(),
			request_id: request_id.clone(),
		}),
		config,
		request_id,
	)
	.await
}

async fn send_message_to_rabbit_mq(
	message: &RequestMessage,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let (channel, rabbitmq_connection) =
		service::get_rabbitmq_connection_channel(config, request_id).await?;

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
