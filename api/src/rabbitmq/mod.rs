use api_models::utils::Uuid;
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
	models::rabbitmq::RequestMessage,
	service,
	utils::Error,
};

mod database;
mod deployment;
mod static_site;

pub async fn start_consumer(app: &App) {
	// Create connection
	let (channel, connection) =
		service::get_rabbitmq_connection_channel(&app.config, &Uuid::new_v4())
			.await
			.expect("unable to get rabbitmq connection");

	// Create Queue
	channel
		.queue_declare(
			app.config.rabbit_mq.queue.as_str(),
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

		let result = process_queue_payload(payload, &app).await;
		let ack_result = if let Err(error) = result {
			log::error!("Error processing payload: {}", error.get_error());
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
}

async fn process_queue_payload(
	content: RequestMessage,
	app: &App,
) -> Result<(), Error> {
	let config = &app.config;
	let mut connection = app.database.acquire().await?;
	match content {
		RequestMessage::Deployment(request_data) => {
			deployment::process_request(&mut connection, request_data, config)
				.await
		}
		RequestMessage::StaticSite(request_data) => {
			static_site::process_request(&mut connection, request_data, config)
				.await
		}
		RequestMessage::Database {} => todo!(),
	}
	.map_err(|error| {
		log::error!("Error processing RabbitMQ message: {}", error.get_error());
		error
	})
}
