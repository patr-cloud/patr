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
use tokio::task;
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
	let body = context.get_body()?;
	let events: EventData = serde_json::from_str(&body)?;
	let config = context.get_state().clone();

	task::spawn(async move {
		let mysql = &mut config.mysql.begin().await.unwrap();
		let config = config.config;
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

			log::info!("Deploying image");
			let repository = target.repository;
			let mut splitter = repository.split('/');
			let org_name = if let Some(val) = splitter.next() {
				val
			} else {
				continue;
			};

			log::info!("Org name: {}", org_name);
			let organisation =
				db::get_organisation_by_name(mysql, org_name).await.unwrap();
			if organisation.is_none() {
				continue;
			}
			let super_admin_id = organisation.unwrap().super_admin_id;

			log::info!("Super admin: {}", hex::encode(&super_admin_id));
			let super_admin = db::get_user_by_user_id(mysql, &super_admin_id)
				.await
				.unwrap();
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
			log::info!("Token generated");
			if token.is_err() {
				continue;
			}
			let token = token.unwrap();

			log::info!("Token unwrapped");
			// get token object using the above token string
			let registry_token = RegistryAuth::builder()
				.username("rakshith-ravi")
				.password("Vicara@124")
				.build();
			let mut stream = docker.images().pull(
				&PullOptions::builder()
					.image(format!("localhost:5000/{}", image_name))
					.auth(registry_token)
					.build(),
			);

			log::info!("Pulling image");
			while let Some(pull_request) = stream.next().await {
				if let Err(err) = pull_request {
					log::error!(
						"Could not pull from the repository. {:#?}",
						err
					);
					continue;
				}

				// can also avoid unwrapping here.
				let pull_request = pull_request.unwrap();

				// now, since the image is pulled, we can go ahead and start the container
				let container_info = docker
					.containers()
					.create(
						&ContainerOptions::builder(&format!(
							"localhost:5000/{}",
							image_name
						))
						.build(),
					)
					.await
					.unwrap();

				log::info!("Got container info");
				let container_id = container_info.id;

				// start the container
				let container_start_result =
					docker.containers().get(&container_id).start().await;

				log::info!("Started container");
				if let Err(err) = container_start_result {
					log::error!(
						"error occured while starting the container. {}",
						&err
					);
					continue;
				}
			}
		}
	});

	Ok(context)
}
