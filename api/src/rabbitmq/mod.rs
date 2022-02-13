use std::ops::DerefMut;

use api_models::{
	models::workspace::infrastructure::deployment::DeploymentStatus,
	utils::Uuid,
};
use futures::{
	future::{self, Either},
	StreamExt,
};
use lapin::{
	options::{
		BasicAckOptions,
		BasicConsumeOptions,
		BasicNackOptions,
		QueueDeclareOptions,
	},
	types::FieldTable,
};
use tokio::{signal, task};

use crate::{
	app::App,
	db,
	models::rabbitmq::{
		DeploymentRequestData,
		RequestMessage,
		StaticSiteRequestData,
	},
	service,
	utils::{settings::Settings, Error},
};

pub async fn start_consumer(app: &App) {
	// Create connection
	let (channel, connection) =
		service::get_rabbitmq_connection_channel(&app.config, &Uuid::new_v4())
			.await
			.expect("unable to get rabbitmq connection");

	// Create Queue
	channel
		.queue_declare(
			"infrastructure",
			QueueDeclareOptions::default(),
			FieldTable::default(),
		)
		.await
		.expect("Cannot create queue");

	let mut consumer = channel
		.basic_consume(
			&app.config.rabbit_mq.queue,
			"patr_queue",
			BasicConsumeOptions::default(),
			FieldTable::default(),
		)
		.await
		.expect("Consumer creation failed");

	let mut shutdown_signal = task::spawn(signal::ctrl_c());
	let mut delivery_future = consumer.next();

	loop {
		println!("Waiting for messages...");
		let selector = future::select(shutdown_signal, delivery_future).await;
		let delivery = match selector {
			Either::Left(_) => {
				break;
			}
			Either::Right((delivery, signal)) => {
				shutdown_signal = signal;
				delivery_future = consumer.next();
				delivery
			}
		};

		let delivery = match delivery {
			Some(Ok(delivery)) => delivery,
			Some(Err(_)) => continue,
			None => panic!("Delivery None"),
		};
		let payload = serde_json::from_slice(&delivery.data);

		let payload = match payload {
			Ok(payload) => payload,
			Err(err) => {
				log::error!(
					"Unknown payload recieved: `{}`",
					String::from_utf8(delivery.data).unwrap_or_default()
				);
				log::error!("Error parsing payload: {}", err);
				continue;
			}
		};

		let result = process_queue_payload(payload, &app.config)
			.await
			.and(
				delivery
					.ack(BasicAckOptions::default())
					.await
					.map_err(|err| err.into()),
			)
			.or({
				delivery
					.nack(BasicNackOptions {
						multiple: false,
						requeue: true,
					})
					.await
			});
		if let Err(error) = result {
			log::error!("Error communicating with rabbitmq: {}", error);
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

async fn process_queue_payload(
	content: RequestMessage,
	config: &Settings,
) -> Result<(), Error> {
	match content {
		RequestMessage::Deployment(request_data) => {
			process_deployment_request(request_data, config).await
		}
		RequestMessage::StaticSite(request_data) => {
			process_static_sites_request(request_data, config).await
		}
		RequestMessage::Database {} => todo!(),
	}
}

async fn process_deployment_request(
	request_data: DeploymentRequestData,
	config: &Settings,
) -> Result<(), Error> {
	match request_data {
		DeploymentRequestData::Update {
			workspace_id,
			deployment,
			full_image,
			running_details,
			request_id,
		} => {
			log::trace!("Received a update kubernetes deployment request");
			service::update_kubernetes_deployment(
				&workspace_id,
				&deployment,
				&full_image,
				&running_details,
				config,
				&request_id,
			)
			.await
			.map_err(|err| {
				log::error!(
					"Error updating kubernetes deployment: {}",
					err.get_error()
				);
				err
			})
		}
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
				service::get_app().database.acquire().await?.deref_mut(),
				&deployment_id,
				&deployment_status,
			)
			.await
			.map_err(|e| e.into())
		}
	}
}

async fn process_static_sites_request(
	request_data: StaticSiteRequestData,
	config: &Settings,
) -> Result<(), Error> {
	match request_data {
		StaticSiteRequestData::Update {
			workspace_id,
			static_site,
			static_site_details,
			request_id,
			static_site_status,
		} => {
			log::trace!("Received a update static site request");
			service::update_kubernetes_static_site(
				&workspace_id,
				&static_site,
				&static_site_details,
				config,
				&request_id,
			)
			.await
			.and({
				log::trace!(
					"request_id: {} - updating database status",
					request_id
				);
				db::update_static_site_status(
					service::get_app().database.acquire().await?.deref_mut(),
					&static_site.id,
					&static_site_status,
				)
				.await
				.map(|_| {
					log::trace!(
						"request_id: {} - updated database status",
						request_id
					);
				})
				.map_err(|err| err.into())
			})
			.or({
				db::update_static_site_status(
					service::get_app().database.acquire().await?.deref_mut(),
					&static_site.id,
					&DeploymentStatus::Errored,
				)
				.await
				.map_err(|err| {
					log::error!(
						"Error occured during deployment of static site: {}",
						err
					);
					err.into()
				})
			})
		}
		StaticSiteRequestData::Delete {
			workspace_id,
			static_site_id,
			request_id,
			static_site_status,
		} => {
			log::trace!("Received a delete static site request");
			service::delete_kubernetes_static_site(
				&workspace_id,
				&static_site_id,
				config,
				&request_id,
			)
			.await?;
			log::trace!(
				"request_id: {} - updating db status to stopped",
				request_id
			);
			db::update_static_site_status(
				service::get_app().database.acquire().await?.deref_mut(),
				&static_site_id,
				&static_site_status,
			)
			.await
			.map_err(|err| err.into())
		}
	}
}
