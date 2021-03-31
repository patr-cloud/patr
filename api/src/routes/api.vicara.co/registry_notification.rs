use std::borrow::Borrow;

use api_macros::closure_as_pinned_box;
use eve_rs::{App as EveApp, Context, Error, NextHandler};
use futures::StreamExt;
use shiplift::{ContainerOptions, Docker, Images, PullOptions, RegistryAuth};
use uuid::Uuid;

use crate::{
	app::{create_eve_app, App},
	async_main, db, error,
	models::db_mapping::EventData,
	models::rbac::{self, permissions},
	models::{RegistryToken, RegistryTokenAccess},
	pin_fn,
	utils::{
		constants::request_keys, get_current_time, validator, EveContext,
		EveMiddleware,
	},
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
	let config = context.get_state().config.clone();

	// check if the event is a push event
	// get image name, repository name, tag if present
	for event in events.events {
		if event.action == "push" {
			let target = event.target;
			if target.tag == "develop" {
				let repository_name = target.repository;
				let username = &context.get_token_data().unwrap().user.username;
				log::info!(
					"Received repo name is {}, and username is {}",
					&repository_name,
					&username
				);

				// init docker`
				let docker = Docker::new();
				let image_name =
					format!("{}:{}", &repository_name, &target.tag);

				// generate token
				let iat = get_current_time().as_secs();
				let token = RegistryToken::new(
					if cfg!(debug_assertions) {
						format!("localhost:{}", config.port)
					} else {
						"api.vicara.co".to_string()
					},
					iat,
					username.to_string(),
					&config,
					vec![RegistryTokenAccess {
						r#type: "repository".to_string(),
						name: repository_name.to_string(),
						actions: vec!["pull".to_string()],
					}],
				)
				.to_string(
					config.docker_registry.private_key.as_ref(),
					config.docker_registry.public_key_der(),
				)?;

				// get token object using the above token string
				let registry_token = RegistryAuth::token(token);
				let mut stream = docker.images().pull(
					&PullOptions::builder()
						.image(&image_name)
						.auth(registry_token)
						.build(),
				);

				while let Some(pull_request) = stream.next().await {
					if let Err(err) = pull_request {
						log::error!(
							"Could not pull from the repository. {}",
							err
						);
						context.status(500).json(error!(SERVER_ERROR));
						return Ok(context);
					}

					// can also avoid unwrapping here.
					let pull_request = pull_request.unwrap();

					// now, since the image is pulled, we can go ahead and start the container
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
					request_keys::SUCCESS : true,
					request_keys::MESSAGE	 : "container running"
				}));
				return Ok(context);
			}
		}
	}

	Ok(context)
}
