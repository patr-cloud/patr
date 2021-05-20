use std::{collections::HashMap, time::Duration};

use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use futures::StreamExt;
use hex::ToHex;
use rand::Rng;
use shiplift::{ContainerOptions, Docker, PullOptions, RegistryAuth};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::{db_mapping::EventData, rbac, RegistryToken, RegistryTokenAccess},
	pin_fn,
	utils::{get_current_time, Error, ErrorData, EveContext, EveMiddleware},
};

pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut sub_app = create_eve_app(app);

	sub_app.post(
		"/docker-registry/notification",
		[EveMiddleware::CustomFunction(pin_fn!(notification_handler))],
	);
	sub_app
}

// TODO: change hard coded port value to deployment's given port.
// use newly refactored table to first fetch port id and then get port number
// from it NOTE: hardcoded port is 3090
pub async fn notification_handler(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	if context.get_content_type().as_str() !=
		"application/vnd.docker.distribution.events.v1+json"
	{
		Error::as_result()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?;
	}
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

		let organisation = db::get_organisation_by_name(
			context.get_mysql_connection(),
			org_name,
		)
		.await?;
		if organisation.is_none() {
			continue;
		}
		let organisation = organisation.unwrap();

		let deployments =
			db::get_deployments_by_image_name_and_tag_for_organisation(
				context.get_mysql_connection(),
				image_name,
				&tag,
				&organisation.id,
			)
			.await?;
		// temporary change
		for deployment in deployments {
			let container_name =
				format!("deployment-{}", deployment.id.encode_hex::<String>());
			let full_image_name = format!(
				"{}/{}/{}@{}",
				config.docker_registry.registry_url,
				org_name,
				deployment.image_name.unwrap(),
				target.digest
			);

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
				config.docker_registry.issuer.clone(),
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
			if let Err(err) = generated_password {
				log::error!("Error generating docker CLI token: {}", err);
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

			let empty_vec = vec![];
			let empty_map = HashMap::new();
			let empty_string = String::default();

			// If the container exists, stop it and delete it
			// The errors will be taken care of by the `unwrap_or_default` part
			let container = docker.containers().get(&container_name);
			let info = container.inspect().await;

			let mut port;

			if let Ok(info) = info {
				// don't redeploy the image if it's already deployed
				if info.config.image == full_image_name {
					log::debug!(
						"Pushed image is already deployed. Ignoring..."
					);
					continue;
				}
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
				port = info
					.host_config
					.port_bindings
					.unwrap_or_default()
					.get(&format!("{}/tcp", 3090))
					.unwrap_or_else(|| &empty_vec)
					.get(0)
					.unwrap_or_else(|| &empty_map)
					.get("HostPort")
					.unwrap_or_else(|| &empty_string)
					.parse()
					.unwrap_or(0);
			} else {
				port = 0;
			}

			if port == 0 {
				// Assign a random, available port
				let low = 1025;
				let high = 65535;
				let restricted_ports = [5800, 8080, 9000, 5000, 3000];
				loop {
					port = rand::thread_rng().gen_range(low..high);
					if restricted_ports.contains(&port) {
						continue;
					}
					let port_open = port_scanner::scan_port_addr(format!(
						"{}:{}",
						"0.0.0.0", port
					));
					if port_open {
						continue;
					}
					break;
				}
			}

			let container = docker
				.containers()
				.create(
					&ContainerOptions::builder(&full_image_name)
						.name(&container_name)
						.privileged(false)
						.expose(3090 as u32, "tcp", port)
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

// TODO: write a refactored version of notification handler.
