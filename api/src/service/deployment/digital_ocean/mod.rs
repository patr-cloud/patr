mod app_deployment;

use std::ops::DerefMut;

pub use app_deployment::*;
use eve_rs::AsError;
use futures::StreamExt;
use reqwest::{Client, Url, header};
use shiplift::{Docker, PullOptions, RegistryAuth};
use tokio::task;

use crate::{
	db,
	models::{
		deployment::cloud_providers::digital_ocean::Auth,
		rbac,
		RegistryToken,
		RegistryTokenAccess,
	},
	service,
	utils::{get_current_time, settings::Settings, Error},
};

pub async fn push_to_digital_ocean_registry(
	image_name: String,
	tag: &str,
	deployment_id: Vec<u8>,
	config: Settings,
) -> Result<(), Error> {
	pull_image_from_registry(&config, &image_name).await?;
	// make a reqwest to push to digital ocean registry
	let image_details = image_name.clone();
	let image_tag = tag.to_string();

	task::spawn(async move {
		let status = push_and_deploy_via_digital_ocean(
			config,
			&deployment_id,
			&image_tag,
			&image_details,
		)
		.await;

		match status {
			Ok(log) => {
				log::info!("{}", log);
			}
			Err(_) => {
				log::info!("Error with the deployment!");
			}
		}
	});

	Ok(())
}

async fn push_and_deploy_via_digital_ocean(
	config: Settings,
	deployment_id: &[u8],
	tag: &str,
	image_name: &str,
) -> Result<String, Error> {
	let auth_token = get_digital_ocean_registry_auth_token(&config).await?;

	let mut headers = header::HeaderMap::new();
	headers.insert("Content-Type", "application/tar".parse().unwrap());
	headers.insert("X-Registry-Auth", auth_token.parse().unwrap());

	let digital_ocean_tag = format!(
		"registry.digitalocean.com/project-apex/{}",
		hex::encode(deployment_id)
	);

	let docker_url = format!(
		"unix:/var/run/docker.sock/v1.41/images/{}/tag?tag={}",
		image_name, digital_ocean_tag
	);	
	let url = Url::parse(&docker_url)?;

	let tag_response = Client::new()
		.post(url)
		.headers(headers.clone())
		.send()
		.await?
		.text()
		.await?;

	let docker_url = format!(
		"unix:/var/run/docker.sock/v1.41/images/{}/push",
		image_name
	);	
	let url = Url::parse(&docker_url)?;

	let push_image = Client::new()
		.post(url)
		.headers(headers.clone())
		.send()
		.await?
		.text()
		.await?;

	if !digital_ocean_app_exists() {
		create_digital_ocean_application(&config, deployment_id, tag).await?;
	}

	return Ok(format!(
		"[TAG STATUS]: {}\n [PUSH STATUS]: {}",
		tag_response, push_image
	));
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

async fn get_digital_ocean_registry_auth_token(
	config: &Settings,
) -> Result<String, Error> {
	let registry = Client::new()
		.post("https://api.digitalocean.com/v2/registry/docker-credentials?read_write=true")
		.bearer_auth(&config.digital_ocean_api_key)
		.send()
		.await?
		.json::<Auth>()
		.await?;

	Ok(registry.registry.auth)
}
