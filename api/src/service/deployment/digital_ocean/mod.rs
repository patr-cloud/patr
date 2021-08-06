mod app_deployment;

use std::{
	ops::DerefMut,
	process::{Command, Stdio},
	str::from_utf8,
};

pub use app_deployment::*;
use eve_rs::AsError;
use futures::StreamExt;
use reqwest::Client;
use shiplift::{Docker, Image, PullOptions, RegistryAuth, TagOptions};
use tokio::task;

use crate::{
	db,
	error,
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
	image_name: &str,
	tag: &str,
	deployment_id: Vec<u8>,
	config: Settings,
) -> Result<(), Error> {
	let image_details = image_name.to_string();
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
			Err(e) => {
				log::info!("Error with the deployment, {:?}", e.get_error());
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
	println!("TEST0");
	pull_image_from_registry(&config, image_name, &tag).await?;
	println!("TEST8");
	// Get login details from digital ocean registry and decode from base 64 to
	// binary
	let auth_token =
		base64::decode(get_digital_ocean_registry_auth_token(&config).await?)?;
	println!("TEST10");
	// Convert auth token from binary to utf8
	let auth_token = from_utf8(&auth_token)?;

	// get username and password from the auth token
	let (username, password) = auth_token
		.split_once(":")
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;
	println!("TEST11");
	let new_repo_name = format!(
		"registry.digitalocean.com/project-apex/{}",
		hex::encode(deployment_id)
	);
	println!("TEST12");
	tag_docker_image(image_name, &new_repo_name, &tag).await?;
	println!("TEST18");
	// Login into the registry
	println!("TEST19");
	let output = Command::new("docker")
		.arg("login")
		.arg("-u")
		.arg(username)
		.arg("-p")
		.arg(password)
		.arg("registry.digitalocean.com")
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()?
		.wait()?;
	println!("TEST20");
	if output.success() {
		let image_push = Command::new("docker")
			.arg("push")
			.arg(format!("registry.digitalocean.com/project-apex/{}",hex::encode(&deployment_id)))
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.spawn()?
			.wait()?;
		println!("TEST21 and image_push:{}", image_push.success());
		if !digital_ocean_app_exists() && image_push.success() {
			create_digital_ocean_application(&config, deployment_id, tag)
				.await?;
			println!("TEST22");
			return Ok(
				"[TAG STATUS]: success [PUSH STATUS]: success".to_string()
			);
		}
	}

	Ok("[TAG STATUS]: success [PUSH STATUS]: failure".to_string())
}

async fn tag_docker_image(
	image_name: &str,
	new_repo_name: &str,
	image_tag: &str,
) -> Result<(), Error> {
	println!("TEST13");
	let docker = Docker::new();
	println!("TEST14");
	let tag_options = TagOptions::builder()
		.repo(new_repo_name)
		.tag(image_tag)
		.build();

	println!("TEST15");
	let image = Image::new(&docker, image_name);
	println!("TEST16");
	image.tag(&tag_options).await?;

	image
		.tag(&tag_options)
		.await
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	println!("TEST17");
	Ok(())
}

async fn pull_image_from_registry(
	config: &Settings,
	image_name: &str,
	tag: &str,
) -> Result<(), Error> {
	println!("TEST1");
	let app = service::get_app().clone();
	let docker = Docker::new();
	println!("TEST2");
	let god_user = db::get_user_by_user_id(
		app.database.acquire().await?.deref_mut(),
		rbac::GOD_USER_ID.get().unwrap().as_bytes(),
	)
	.await?
	.status(500)?;
	println!("TEST3");
	let god_username = god_user.username;
	println!("TEST4");
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
	println!("TEST5");
	// get token object using the above token string
	let registry_auth = RegistryAuth::builder()
		.username(god_username)
		.password(token)
		.build();
	println!("TEST6");
	let mut stream = docker.images().pull(
		&PullOptions::builder()
			.image(format!("{}:{}", &image_name, tag))
			.auth(registry_auth)
			.build(),
	);
	println!("TEST7");
	while stream.next().await.is_some() {}

	Ok(())
}

pub fn digital_ocean_app_exists() -> bool {
	false
}

async fn get_digital_ocean_registry_auth_token(
	config: &Settings,
) -> Result<String, Error> {
	println!("TEST9");
	let registry = Client::new()
		.get("https://api.digitalocean.com/v2/registry/docker-credentials?read_write=true?expiry_seconds=86400")
		.bearer_auth(&config.digital_ocean_api_key)
		.send()
		.await?
		.json::<Auth>()
		.await?;

	Ok(registry.auths.registry.auth)
}
