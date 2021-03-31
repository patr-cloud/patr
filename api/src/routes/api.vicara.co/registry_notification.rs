use std::{borrow::Borrow, str::FromStr};

use api_macros::closure_as_pinned_box;
use eve_rs::{App as EveApp, Context, Error, NextHandler};
use futures::StreamExt;
use shiplift::{
	ContainerOptions,
	Docker,
	Images,
	PullOptions,
	RegistryAuth,
	Uri,
};
use uuid::Uuid;

use crate::{
	app::{create_eve_app, App},
	async_main,
	db,
	error,
	models::{
		db_mapping::EventData,
		rbac::{self, permissions},
		RegistryToken,
		RegistryTokenAccess,
	},
	pin_fn,
	utils::{
		constants::request_keys,
		get_current_time,
		validator,
		EveContext,
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
		if event.action != "push" {
			continue;
		}
		let target = event.target;
		if target.tag.is_empty() {
			continue;
		}

		let repository = target.repository;
		let mut splitter = repository.split('/');
		let org_name = if let Some(val) = splitter.next() {
			val
		} else {
			continue;
		};

		let organisation = db::get_organisation_by_name(
			context.get_mysql_connection(),
			org_name,
		)
		.await?;
		if organisation.is_none() {
			continue;
		}
		let super_admin_id = organisation.unwrap().super_admin_id;

		let super_admin = db::get_user_by_user_id(
			context.get_mysql_connection(),
			&super_admin_id,
		)
		.await?;
		if super_admin.is_none() {
			continue;
		}
		let super_admin = super_admin.unwrap();

		// init docker
		let docker = Docker::new();
		let image_name = format!("{}:{}", &repository, &target.tag);

		// generate token
		let iat = get_current_time().as_secs();
		let token = RegistryToken::new(
			if cfg!(debug_assertions) {
				format!("localhost:{}", config.port)
			} else {
				"api.vicara.co".to_string()
			},
			iat,
			super_admin.username,
			&config,
			vec![RegistryTokenAccess {
				r#type: "repository".to_string(),
				name: repository.to_string(),
				actions: vec!["pull".to_string()],
			}],
		)
		.to_string(
			config.docker_registry.private_key.as_ref(),
			config.docker_registry.public_key_der(),
		);
		if token.is_err() {
			continue;
		}
		let token = token.unwrap();

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
				log::error!("Could not pull from the repository. {}", err);
				continue;
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
				continue;
			}
		}
	}

	Ok(context)
}
