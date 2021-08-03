mod app_deployment;

use std::ops::DerefMut;

pub use app_deployment::*;
use eve_rs::AsError;
use futures::StreamExt;
use reqwest::{header, Client};
use shiplift::{Docker, PullOptions, RegistryAuth};
use tokio::task;

use crate::{
	db,
	models::{rbac, RegistryToken, RegistryTokenAccess},
	service,
	utils::{get_current_time, settings::Settings, Error},
};

pub async fn push_to_digital_ocean_registry(
	image_name: String,
	deployment_id: Vec<u8>,
	config: Settings,
) -> Result<(), Error> {
	pull_image_from_registry(&config, &image_name).await?;
	// make a reqwest to push to digital ocean registry
	task::spawn(async move {
		// First tag the image
		let encoded_string = base64::encode(
			"{
				\"username\": \"string\", 
				\"password\": \"string\", 
				\"serveraddress\": \"registry.digitalocean.com\"
			}",
		);
		let mut headers = header::HeaderMap::new();
		// Find alternative of unwrap
		headers.insert("Content-Type", "application/tar".parse().unwrap());
		headers.insert("X-Registry-Auth", encoded_string.parse().unwrap());

		let digital_ocean_tag = format!(
			"registry.digitalocean.com/project-apex/{}",
			hex::encode(deployment_id)
		);

		let tag_response = Client::new()
			.post(format!(
				"http://localhost/v1.41/images/{}/tag?tag={}",
				image_name, digital_ocean_tag
			))
			.headers(headers.clone())
			.send()
			.await;
		// Do something about the tag status and add if successful conditions

		let push_image = Client::new()
			.post(format!(
				"http://localhost/v1.41/images/{}/push",
				digital_ocean_tag
			))
			.headers(headers.clone())
			.send()
			.await;
	});

	if !digital_ocean_app_exists() {
		create_digital_ocean_application(&config, deployment_id, tag).await?;
	}

	Ok(())
}

async fn pull_image_from_registry(
	config: &Settings,
	image_name: &str,
) -> Result<(), Error> {
	let app = service::get_app().clone();
	let docker = Docker::new();

	let god_user = db::get_user_by_user_id(
		app.database.acquire().await?.deref_mut(),
		rbac::GOD_USER_ID.get().unwrap().as_bytes(),
	)
	.await?
	.status(500)?;
	let god_username = god_user.username;

	// generate token as password
	let iat = get_current_time().as_secs();
	let token = RegistryToken::new(
		config.docker_registry.issuer.clone(),
		iat,
		god_username.clone(),
		config,
		vec![RegistryTokenAccess {
			r#type: "repository".to_string(),
			name: image_name.to_string(),
			actions: vec!["pull".to_string()],
		}],
	)
	.to_string(
		config.docker_registry.private_key.as_ref(),
		config.docker_registry.public_key_der(),
	)?;
	// get token object using the above token string
	let registry_auth = RegistryAuth::builder()
		.username(god_username)
		.password(token)
		.build();
	let mut stream = docker.images().pull(
		&PullOptions::builder()
			.image(&image_name.to_string())
			.auth(registry_auth)
			.build(),
	);

	while stream.next().await.is_some() {}

	Ok(())
}

pub fn digital_ocean_app_exists() -> bool {
	false
}
