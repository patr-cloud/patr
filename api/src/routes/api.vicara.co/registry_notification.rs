use std::borrow::Borrow;

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
			if target.tag == "develop" {
				let mut repository_name = target.repository;

				// pull the image
				let host = "localhost";
				let port: i32 = 5000;
				let username = "username";
				let password = "password";

				// init docker`
				let docker = Docker::new();
				let mut image_name =
					format!("{}/{}", &repository_name, &target.tag);

				let auth = RegistryAuth::builder()
					.username(&username.to_string())
					.password(&password.to_string())
					.build();

				let mut stream = docker.images().pull(
					&PullOptions::builder()
						.image(&image_name)
						.auth(auth)
						.build(),
				);

				while let Some(pull_request) = stream.next().await {
					if let Err(err) = pull_request {
						context.status(500);
						context.json(json!({
							"success": false,
						}));
						return Ok(context);
					}

					let pull_request = pull_request.unwrap();
					// now since the image is pulled, we can go ahead and start the container
					let container_info = docker
						.containers()
						.create(&ContainerOptions::builder(&image_name).build())
						.await?;

					let container_id = container_info.id;

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
							"Success": false
						}));

						return Ok(context);
					}
				}

				// if here, then the container is successfully running.
				context.status(200).json(json!({
					"success" : true,
					"message" : "container running"
				}));
				return Ok(context);
			}
		}
	}

	Ok(context)
}
