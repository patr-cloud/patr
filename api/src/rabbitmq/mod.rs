use std::ops::DerefMut;

use api_models::models::workspace::infrastructure::deployment::DeploymentStatus;
use eve_rs::AsError;
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
	db,
	error,
	models::rabbitmq::{
		DeploymentRequestData,
		RequestData,
		RequestMessage,
		RequestType,
		StaticSiteRequestData,
	},
	service,
	utils::{settings::Settings, Error},
};

pub async fn start_consumer(config: &Settings) {
	// Create connection
	let connection = Connection::connect(
		&format!(
			"amqp://{}:{}/%2f",
			config.rabbit_mq.host, config.rabbit_mq.port
		),
		ConnectionProperties::default(),
	)
	.await
	.expect("Cannot establish connection to RabbitMQ");

	let channel = connection
		.create_channel()
		.await
		.expect("Cannot create channel");

	// Create Queue
	let _ = channel
		.queue_declare(
			"infrastructure",
			QueueDeclareOptions::default(),
			FieldTable::default(),
		)
		.await
		.expect("Cannot create queue");

	let mut shutdown_signal = task::spawn(signal::ctrl_c());

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
	println!("Shutting down consumer");
}

async fn execute_kubernetes_deployment(
	content: RequestMessage,
) -> Result<(), Error> {
	match content.request_type {
		RequestType::Update => {
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
							.await
						}
						_ => {
							log::error!("Unable to deserialize request data");
							// TODO: change the return statement
							Error::as_result()
								.status(500)
								.body(error!(SERVER_ERROR).to_string())
						}
					}
				}
				RequestData::StaticSiteRequest(static_site_request_data) => {
					match *static_site_request_data {
						StaticSiteRequestData::Update {
							workspace_id,
							static_site,
							static_site_details,
							config,
							request_id,
						} => {
							log::trace!(
								"Received a update static site request"
							);
							let result =
								service::update_kubernetes_static_site(
									&workspace_id,
									&static_site,
									&static_site_details,
									&config,
									&request_id,
								)
								.await;

							match result {
								Ok(()) => {
									log::trace!(
										"request_id: {} - updating database status",
										request_id
									);
									log::trace!("request_id: {} - updated database status", request_id);
									db::update_static_site_status(
										service::get_app()
											.database
											.acquire()
											.await?
											.deref_mut(),
										&static_site.id,
										&DeploymentStatus::Running,
									)
									.await
									.map_err(|err| err.into())
								}
								Err(e) => {
									log::error!(
										"Error occured during deployment of static site: {}",
										e.get_error()
									);
									db::update_static_site_status(
										service::get_app()
											.database
											.acquire()
											.await?
											.deref_mut(),
										&static_site.id,
										&DeploymentStatus::Errored,
									)
									.await
									.map_err(|err| err.into())
								}
							}
						}
						_ => {
							log::error!("Unable to deserialize request data");
							Error::as_result()
								.status(500)
								.body(error!(SERVER_ERROR).to_string())
						}
					}
				}
				RequestData::DatabaseRequest {} => todo!(),
			}
		}
		RequestType::Delete => {
			match content.request_data {
				RequestData::Deployment(deployment_request_data) => {
					match *deployment_request_data {
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

							// TODO: change this incase the request is stop
							// deployment
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
							.map_err(|e| e.into())
						}
						_ => Error::as_result()
							.status(500)
							.body(error!(SERVER_ERROR).to_string()),
					}
				}
				RequestData::StaticSiteRequest(static_site_request_data) => {
					match *static_site_request_data {
						StaticSiteRequestData::Delete {
							workspace_id,
							static_site_id,
							config,
							request_id,
						} => {
							log::trace!(
								"Received a delete static site request"
							);
							service::delete_kubernetes_static_site(
								&workspace_id,
								&static_site_id,
								&config,
								&request_id,
							)
							.await?;
							log::trace!("request_id: {} - updating db status to stopped", request_id);
							db::update_static_site_status(
								service::get_app()
									.database
									.acquire()
									.await?
									.deref_mut(),
								&static_site_id,
								&DeploymentStatus::Stopped,
							)
							.await
							.map_err(|err| err.into())
						}
						_ => Error::as_result()
							.status(500)
							.body(error!(SERVER_ERROR).to_string()),
					}
				}
				RequestData::DatabaseRequest {} => todo!(),
			}
		}

		_ => Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?,
	}
}
