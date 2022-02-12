use std::ops::DerefMut;

use api_models::{
	models::workspace::infrastructure::deployment::DeploymentStatus,
	utils::Uuid,
};
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
	let (channel, connection) =
		service::get_rabbitmq_connection_channel(config, &Uuid::new_v4())
			.await
			.unwrap();

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
		println!("Waiting for messages...");
		let delivery = match consumer.next().await {
			Some(Ok(delivery)) => delivery,
			Some(Err(_)) => continue,
			None => panic!("Delivery failed"),
		};
		let content = delivery.data.clone();
		let payload = serde_json::from_slice(content.as_slice());

		let payload = match payload {
			Ok(payload) => payload,
			Err(err) => {
				println!("content: {}", String::from_utf8(content).unwrap());
				log::error!("{}", err);
				continue;
			}
		};

		let response = execute_kubernetes_deployment(payload, config).await;
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

	channel
		.close(200, "closing channel")
		.await
		.expect("Channel close failed");
	connection
		.close(200, "Bye")
		.await
		.expect("Connection close failed");
	println!("Shutting down consumer");
}

async fn execute_kubernetes_deployment(
	content: RequestMessage,
	config: &Settings,
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
							request_id,
						} => {
							log::trace!("Received a update kubernetes deployment request");
							let update_kubernetes_result =
								service::update_kubernetes_deployment(
									&workspace_id,
									&deployment,
									&full_image,
									&running_details,
									config,
									&request_id,
								)
								.await;

							if let Err(err) = update_kubernetes_result {
								log::error!(
									"Error updating kubernetes deployment: {}",
									err.get_error()
								);
								Err(err)
							} else {
								Ok(())
							}
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
							request_id,
							static_site_status,
						} => {
							log::trace!(
								"Received a update static site request"
							);
							let result =
								service::update_kubernetes_static_site(
									&workspace_id,
									&static_site,
									&static_site_details,
									config,
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
										&static_site_status,
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
							request_id,
							deployment_status,
						} => {
							log::trace!("reciver a delete kubernetes deployment request");
							service::delete_kubernetes_deployment(
								&workspace_id,
								&deployment_id,
								config,
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
								&deployment_status,
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
							request_id,
							static_site_status,
						} => {
							log::trace!(
								"Received a delete static site request"
							);
							service::delete_kubernetes_static_site(
								&workspace_id,
								&static_site_id,
								config,
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
								&static_site_status,
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
	}
}
