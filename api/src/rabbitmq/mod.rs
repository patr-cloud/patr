use std::ops::DerefMut;

use api_models::models::workspace::infrastructure::deployment::DeploymentStatus;
use futures::{FutureExt, StreamExt};
use lapin::{
	options::{
		BasicAckOptions,
		BasicConsumeOptions,
		BasicPublishOptions,
		QueueDeclareOptions,
	},
	types::FieldTable,
	BasicProperties,
	Connection,
	ConnectionProperties,
};
use tokio::{signal, task};

use crate::{
	app::RabbitMqConnection,
	db,
	models::rabbitmq::{
		DeploymentRequestData,
		RequestData,
		RequestMessage,
		RequestType,
	},
	service,
	utils::{settings::Settings, Error},
};

pub async fn set_up_rabbitmq(
	config: &Settings,
) -> Result<RabbitMqConnection, Error> {
	// Create a connection to RabbitMQ
	let connection = Connection::connect(
		&format!(
			"amqp://{}:{}/%2f",
			config.rabbit_mq.host, config.rabbit_mq.port
		),
		ConnectionProperties::default(),
	)
	.await?;

	// Create channel
	let channel_a = connection.create_channel().await?;
	let channel_b = connection.create_channel().await?;

	// Create Queue
	let _ = channel_a
		.queue_declare(
			"infrastructure",
			QueueDeclareOptions::default(),
			FieldTable::default(),
		)
		.await?;

	Ok(RabbitMqConnection {
		channel_a,
		channel_b,
		queue: config.rabbit_mq.queue.clone(),
	})
}

pub async fn start_consumer(config: &Settings, channel: lapin::Channel) {
	let mut shutdown_signal = task::spawn(signal::ctrl_c());

	log::trace!("Creating a consumer");
	let mut consumer = channel
		.basic_consume(
			&config.rabbit_mq.queue,
			"patr_queue",
			BasicConsumeOptions::default(),
			FieldTable::default(),
		)
		.await
		.expect("Consumer creation failed");

	while (&mut shutdown_signal).now_or_never().is_none() {
		let delivery = match consumer.next().await {
			Some(Ok(delivery)) => delivery,
			Some(Err(_)) => continue,
			None => panic!("Delivery failed"),
		};
		let content = delivery.data.clone();
		let payload = serde_json::from_slice(content.as_slice());

		let payload = if let Ok(payload) = payload {
			payload
		} else {
			log::error!("Unable to deserialize request message");
			return;
		};

		let response = execute_kubernetes_deployment(payload).await;
		if response.is_ok() {
			if let Err(err) = delivery.ack(BasicAckOptions::default()).await {
				log::error!("Unable to ack message: {}", err);
			};
		} else {
			let publish_result = channel
				.basic_publish(
					"",
					"infrastructure",
					BasicPublishOptions::default(),
					&content,
					BasicProperties::default(),
				)
				.await;

			let _result = match publish_result {
				Ok(publish_result) => match publish_result.await {
					Ok(result) => result,
					Err(e) => {
						log::error!("Error consuming message from infrastructure queue: {}", e);
						return;
					}
				},
				Err(e) => {
					log::error!(
						"Error consuming message  infrastructure queue: {}",
						e
					);
					return;
				}
			};
		}
	}
}

async fn execute_kubernetes_deployment(
	content: RequestMessage,
) -> Result<(), Error> {
	match content.request_type {
		RequestType::Create => {
			match content.request_data {
				RequestData::Deployment(deployment_request_data) => {
					match *deployment_request_data {
						DeploymentRequestData::Update {
							workspace_id,
							deployment,
							full_image,
							running_details,
							config,
							request_id,
						} => {
							log::trace!("Received a update kubernetes deployment request");
							service::update_kubernetes_deployment(
								&workspace_id,
								&deployment,
								&full_image,
								&running_details,
								&config,
								&request_id,
							)
							.await?
						}
						DeploymentRequestData::Delete {
							workspace_id,
							deployment_id,
							config,
							request_id,
						} => {
							log::trace!("reciver a delete kubernetes deployment request");
							service::delete_kubernetes_deployment(
								&workspace_id,
								&deployment_id,
								&config,
								&request_id,
							)
							.await?;

							db::update_deployment_status(
								service::get_app()
									.database
									.acquire()
									.await?
									.deref_mut(),
								&deployment_id,
								&DeploymentStatus::Deleted,
							)
							.await
							.map_err(|e| {
								log::error!(
									"Error updating deployment status: {}",
									e
								);
								e
							})?;
						}
					}
				}
				RequestData::StaticSiteRequest {} => todo!(),
				RequestData::DatabaseRequest {} => todo!(),
			}
		}
		RequestType::Update => match &content.request_data {
			RequestData::Deployment(_) => todo!(),
			RequestData::StaticSiteRequest {} => todo!(),
			RequestData::DatabaseRequest {} => todo!(),
		},
		RequestType::Delete => match &content.request_data {
			RequestData::Deployment(_) => todo!(),
			RequestData::StaticSiteRequest {} => todo!(),
			RequestData::DatabaseRequest {} => todo!(),
		},
		RequestType::Get => match &content.request_data {
			RequestData::Deployment(_) => todo!(),
			RequestData::StaticSiteRequest {} => todo!(),
			RequestData::DatabaseRequest {} => todo!(),
		},
		// IF DEPLOYMENT, CALL SEPARATE FUNCTION
	};
	Ok(())
}
