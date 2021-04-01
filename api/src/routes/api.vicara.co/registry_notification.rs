use std::time::Duration;

use eve_rs::{App as EveApp, Context, Error, NextHandler};
use futures::StreamExt;
use shiplift::{ContainerOptions, Docker, PullOptions, RegistryAuth};

use crate::{
	app::{create_eve_app, App},
	db,
	models::{db_mapping::EventData, rbac, RegistryToken, RegistryTokenAccess},
	pin_fn,
	utils::{get_current_time, EveContext, EveMiddleware},
};

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
	let config = context.get_state().clone().config;

	// init docker
	let docker = Docker::new();

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
		let image_name = if let Some(val) = splitter.next() {
			val
		} else {
			continue;
		};
		let tag = target.tag;

		let deployments = db::get_deployments_by_image_name_and_tag(
			context.get_mysql_connection(),
			image_name,
			&tag,
		)
		.await?;

		for deployment in deployments {
			let container_name =
				format!("deployment-{}", hex::encode(&deployment.id));
			let full_image_name = format!(
				"{}/{}/{}:{}",
				config.docker_registry.registry_url,
				org_name,
				deployment.image_name,
				deployment.image_tag,
			);

			// If the container exists, stop it and delete it
			// The errors will be taken care of by the `unwrap_or_default` part
			docker
				.containers()
				.get(&container_name)
				.stop(Some(Duration::from_secs(5)))
				.await
				.unwrap_or_default();
			docker
				.containers()
				.get(&container_name)
				.delete()
				.await
				.unwrap_or_default();

			// Pull the latest image again
			let god_user = db::get_user_by_user_id(
				context.get_mysql_connection(),
				rbac::GOD_USER_ID.get().unwrap().as_bytes(),
			)
			.await?
			.unwrap();
			let god_username = god_user.username;
			// generate token as password
			let iat = get_current_time().as_secs();
			let generated_password = RegistryToken::new(
				if cfg!(debug_assertions) {
					format!("localhost:{}", config.port)
				} else {
					"api.vicara.co".to_string()
				},
				iat,
				god_username.clone(),
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
			if generated_password.is_err() {
				continue;
			}
			let token = generated_password.unwrap();

			// get token object using the above token string
			let registry_auth = RegistryAuth::builder()
				.username(god_username)
				.password(token)
				.build();
			let mut stream = docker.images().pull(
				&PullOptions::builder()
					.image(&full_image_name)
					.auth(registry_auth)
					.build(),
			);
			while let Some(_) = stream.next().await {}

			let container = docker
				.containers()
				.create(
					&ContainerOptions::builder(&full_image_name)
						.name(&container_name)
						.privileged(false)
						.expose(deployment.port as u32, "tcp", 8080)
						.build(),
				)
				.await;
			if let Err(err) = container {
				log::error!("Error creating container: {:?}", err);
				// TODO log somewhere that the creation failed and that it
				// needs to be done again
				continue;
			}

			let result = docker.containers().get(&container_name).start().await;
			if let Err(err) = result {
				log::error!("Error starting container: {:?}", err);
				// TODO log somewhere that the start failed and that it
				// needs to be done again
				continue;
			}
		}
	}

	Ok(context)
}
