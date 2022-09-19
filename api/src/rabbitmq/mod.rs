use deadpool::managed::Object;
use deadpool_lapin::{Config, Manager, Pool, Runtime};
use futures::{
	future::{self, Either},
	StreamExt,
};
use lapin::{
	options::{
		BasicAckOptions,
		BasicConsumeOptions,
		BasicNackOptions,
		ConfirmSelectOptions,
		QueueDeclareOptions,
	},
	types::FieldTable,
	Channel,
};
use tokio::{signal, task};

use crate::{
	app::App,
	models::rabbitmq::{
		CIData,
		DeploymentRequestData,
		Queue,
		WorkspaceRequestData,
	},
	utils::{settings::Settings, Error},
};

mod billing;
mod byoc;
mod ci;
mod database;
mod deployment;

pub use ci::{BuildId, BuildStep, BuildStepId};

pub async fn start_consumer(app: &App) {
	future::join_all(Queue::iterator().map(|queue| {
		let app = app.clone();
		tokio::spawn(async move {
			let (channel, connection) =
				get_rabbitmq_connection_channel(&app.rabbitmq)
					.await
					.expect("unable to get rabbitmq connection");
			// Create Queue
			channel
				.queue_declare(
					&queue.to_string(),
					QueueDeclareOptions::default(),
					FieldTable::default(),
				)
				.await
				.expect("Cannot create queue");

			let mut consumer = channel
				.basic_consume(
					&queue.to_string(),
					&queue.to_string(),
					BasicConsumeOptions::default(),
					FieldTable::default(),
				)
				.await
				.expect("Consumer creation failed");

			let mut shutdown_signal = task::spawn(signal::ctrl_c());
			let mut delivery_future = consumer.next();

			loop {
				println!("{} queue waiting for messages...", queue);
				let selector =
					future::select(shutdown_signal, delivery_future).await;
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

				let result = match queue {
					Queue::Infrastructure => {
						let payload = serde_json::from_slice::<
							DeploymentRequestData,
						>(&delivery.data);

						let payload =
							match payload {
								Ok(payload) => payload,
								Err(err) => {
									log::error!(
										"Unknown payload recieved: `{}`",
										String::from_utf8(delivery.data)
											.unwrap_or_default()
									);
									log::error!("Error parsing payload: {} for infra queue", err);
									continue;
								}
							};
						process_infra_queue_payload(payload, &app).await
					}
					Queue::Ci => {
						let payload =
							serde_json::from_slice::<CIData>(&delivery.data);

						let payload =
							match payload {
								Ok(payload) => payload,
								Err(err) => {
									log::error!(
										"Unknown payload recieved: `{}`",
										String::from_utf8(delivery.data)
											.unwrap_or_default()
									);
									log::error!("Error parsing payload: {} for CI queue", err);
									continue;
								}
							};
						process_ci_queue_payload(payload, &app).await
					}
					Queue::Billing => {
						let payload = serde_json::from_slice::<
							WorkspaceRequestData,
						>(&delivery.data);

						let payload = match payload {
							Ok(payload) => payload,
							Err(err) => {
								log::error!(
									"Unknown payload recieved: `{}`",
									String::from_utf8(delivery.data)
										.unwrap_or_default()
								);
								log::error!("Error parsing payload: {}  for workspace queue", err);
								continue;
							}
						};
						process_billing_queue_payload(payload, &app).await
					}
				};
				let ack_result = if let Err(error) = result {
					log::error!(
						"Error processing payload: {}",
						error.get_error()
					);
					delivery
						.nack(BasicNackOptions {
							multiple: false,
							requeue: true,
						})
						.await
				} else {
					delivery.ack(BasicAckOptions::default()).await
				};

				if let Err(error) = ack_result {
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
		})
	}))
	.await
	.into_iter()
	.collect::<Result<Vec<_>, _>>()
	.expect("Error occurred while spawing a task");
}

async fn process_infra_queue_payload(
	data: DeploymentRequestData,
	app: &App,
) -> Result<(), Error> {
	let config = &app.config;
	let mut connection = app.database.acquire().await?;
	deployment::process_request(&mut connection, data, config)
		.await
		.map_err(|error| {
			log::error!(
				"Error processing infra RabbitMQ message: {}",
				error.get_error()
			);
			error
		})
}

async fn process_ci_queue_payload(
	data: CIData,
	app: &App,
) -> Result<(), Error> {
	let config = &app.config;
	let mut connection = app.database.acquire().await?;
	ci::process_request(&mut connection, data, config)
		.await
		.map_err(|error| {
			log::error!(
				"Error processing CI RabbitMQ message: {}",
				error.get_error()
			);
			error
		})
}

async fn process_billing_queue_payload(
	data: WorkspaceRequestData,
	app: &App,
) -> Result<(), Error> {
	let config = &app.config;
	let mut connection = app.database.acquire().await?;
	billing::process_request(&mut connection, data, config)
		.await
		.map_err(|error| {
			log::error!(
				"Error processing bills RabbitMQ message: {}",
				error.get_error()
			);
			error
		})
}

pub(super) async fn create_rabbitmq_pool(
	config: &Settings,
) -> Result<Pool, Error> {
	let cfg = Config {
		url: Some(format!(
			"amqp://{}:{}@{}:{}/%2f",
			config.rabbit_mq.username,
			config.rabbit_mq.password,
			config.rabbit_mq.host,
			config.rabbit_mq.port
		)),
		..Config::default()
	};
	let pool = cfg.create_pool(Some(Runtime::Tokio1))?;

	Ok(pool)
}

pub(super) async fn get_rabbitmq_connection_channel(
	pool: &Pool,
) -> Result<(Channel, Object<Manager>), Error> {
	let connection = pool.get().await?;
	let channel = connection.create_channel().await?;

	channel
		.confirm_select(ConfirmSelectOptions::default())
		.await?;

	Ok((channel, connection))
}
