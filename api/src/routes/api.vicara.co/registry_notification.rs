use api_macros::closure_as_pinned_box;
use eve_rs::{App as EveApp, Context, Error, NextHandler};
use futures::StreamExt;
use shiplift::{ContainerOptions, Docker, Images, PullOptions, RegistryAuth};

use crate::{
	app::{create_eve_app, App},
	async_main, db, error,
	models::db_mapping::EventData,
	models::rbac::{self, permissions},
	pin_fn,
	utils::{constants::request_keys, validator, EveContext, EveMiddleware},
};
use serde_json::{json, Deserializer, Value};

pub fn create_sub_app(app: &App) -> EveApp<EveContext, EveMiddleware, App> {
	let mut sub_app = create_eve_app(app);

	sub_app.post(
		"/notification",
		&[EveMiddleware::CustomFunction(pin_fn!(notification_handler))],
	);
	sub_app
}

pub async fn notification_handler(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let body = context.get_body_object().clone();
	let events: EventData = serde_json::from_value(body)?;

	// check if the event is a push event
	// get image name, repository name, tag if present
	for event in events.events {
		if event.action == "push" {
			let target = event.target;
			let tag = target.tag;
			if tag == "develop" {
				let mut repository_name = target.repository;

				// pull the image
				let host = "localhost";
				let port = 5000;
				let mut image_name = format!(
					"{}:{}/{}/{}",
					&host, &port, &repository_name, &tag
				);

				let docker = Docker::new();
				log::info!("Detected develop tag..");

				// // let image = docker.images().new(&docker);
				// let image = Images::new(&docker);
				// let pull_options =
				// 	PullOptions::builder().image(&image_name).build();

				// pull image
				// let mut something = image.pull(&pull_options);

				// while let Some(value) = something.next().await {
				// 	//TODO: handle error for this
				// 	if let Err(err) = value {
				// 		context.status(500);
				// 		return Ok(context);
				// 	}
				// 	let result = value.unwrap();
				// 	log::debug!("received info")
				// }

				let container_info = docker
					.containers()
					.create(&ContainerOptions::builder(&image_name).build())
					.await?;

				let container_id = container_info.id;

				log::info!("Container info created...");
				log::info!("Starting docker container...");
				// start the container
				let container_start_result =
					docker.containers().get(&container_id).start().await;

				if let Err(err) = container_start_result {
					log::error!(
						"error occured while starting the container. {}",
						&err
					);
					context.status(500);
					context.json(json!({
						"Success": "failed"
					}));

					return Ok(context);
				}

				log::info!("docker container running");
				log::info!("DOING SOMETHING...");
			}
		}
	}

	Ok(context)
}

pub async fn connect_check(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	Ok(context)
}
