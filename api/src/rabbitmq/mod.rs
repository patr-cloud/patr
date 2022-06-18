use api_models::utils::Uuid;
use chrono::{Datelike, Utc};
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
	db::Workspace,
	models::rabbitmq::{RequestMessage, WorkspaceRequestData},
	service,
	utils::{settings::Settings, Error},
};

mod billing;
mod database;
mod deployment;
mod static_site;

pub async fn start_consumer(app: &App) {
	// Create connection
	let (channel, connection) = get_rabbitmq_connection_channel(&app.rabbitmq)
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

		let result = process_queue_payload(payload, app).await;
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
		RequestMessage::Workspace(request_data) => {
			billing::process_request(&mut connection, request_data, config)
				.await
		}
	}
	.map_err(|error| {
		log::error!("Error processing RabbitMQ message: {}", error.get_error());
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

pub(super) async fn queue_process_payment(
	config: &Settings,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();

	let current_month = Utc::now().month();
	let current_year = Utc::now().year();

	service::send_message_to_rabbit_mq(
		&RequestMessage::Workspace(WorkspaceRequestData::ProcessWorkspaces {
			month: current_month,
			year: current_year,
			request_id: request_id.clone(),
		}),
		config,
		&request_id,
	)
	.await
}

pub(super) async fn queue_confirm_payment_intent(
	config: &Settings,
	payment_intent_id: String,
	workspace_id: Uuid,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();

	service::send_message_to_rabbit_mq(
		&RequestMessage::Workspace(
			WorkspaceRequestData::ConfirmPaymentIntent {
				payment_intent_id,
				workspace_id,
				request_id: request_id.clone(),
			},
		),
		config,
		&request_id,
	)
	.await
}

pub(super) async fn queue_generate_invoice_for_workspace(
	config: &Settings,
	workspace: Workspace,
	month: u32,
	year: i32,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();

	service::send_message_to_rabbit_mq(
		&RequestMessage::Workspace(WorkspaceRequestData::GenerateInvoice {
			month: 6,
			year: 2022,
			workspace,
			request_id: request_id.clone(),
		}),
		config,
		&request_id,
	)
	.await
}
