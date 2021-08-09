use std::{ops::DerefMut, process::Stdio, str};

use eve_rs::AsError;
use futures::StreamExt;
use reqwest::Client;
use shiplift::{Docker, PullOptions, RegistryAuth, TagOptions};
use tokio::process::Command;

use crate::{
	db,
	error,
	models::{
		deployment::cloud_providers::digital_ocean::{
			AppConfig,
			AppHolder,
			AppSpec,
			Auth,
			Domains,
			Image,
			Routes,
			Services,
		},
		rbac,
		RegistryToken,
		RegistryTokenAccess,
	},
	service,
	utils::{get_current_time, settings::Settings, Error},
};

pub async fn deploy_container_on_digitalocean(
	image_name: String,
	tag: String,
	deployment_id: Vec<u8>,
	config: Settings,
) -> Result<(), Error> {
	pull_image_from_registry(&image_name, &tag, &config).await?;

	// new name for the docker image
	let new_repo_name = format!(
		"registry.digitalocean.com/project-apex/{}",
		hex::encode(&deployment_id)
	);

	// rename the docker image with the digital ocean registry url
	tag_docker_image(&image_name, &new_repo_name).await?;

	// Get login details from digital ocean registry and decode from base 64 to
	// binary
	let auth_token = base64::decode(get_registry_auth_token(&config).await?)?;

	// Convert auth token from binary to utf8
	let auth_token = str::from_utf8(&auth_token)?;

	// get username and password from the auth token
	let (username, password) = auth_token
		.split_once(":")
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	// Login into the registry
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
		.wait()
		.await?;

	if !output.success() {
		return Err(Error::empty()
			.status(500)
			.body(error!(SERVER_ERROR).to_string()));
	}

	// if the loggin in is successful the push the docker image to registry
	let push_status = Command::new("docker")
		.arg("push")
		.arg(format!(
			"registry.digitalocean.com/project-apex/{}",
			hex::encode(&deployment_id)
		))
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()?
		.wait()
		.await?;

	if !push_status.success() {
		return Err(Error::empty()
			.status(500)
			.body(error!(SERVER_ERROR).to_string()));
	}

	// if the app exists then only create a deployment
	let app_exists = app_exists(&deployment_id, &config).await?;

	if let Some(app_id) = app_exists {
		// the function to create a new deployment
		redeploy_application(&app_id, &config).await?;

		Ok(())
	} else {
		// if the app doesn't exists then create a new app
		create_app(&deployment_id, &tag, &config).await?;

		// TODO update DNS

		Ok(())
	}
}

async fn tag_docker_image(
	image_name: &str,
	new_repo_name: &str,
) -> Result<(), Error> {
	let docker = Docker::new();

	docker
		.images()
		.get(image_name)
		.tag(
			&TagOptions::builder()
				.repo(new_repo_name)
				.tag("latest")
				.build(),
		)
		.await?;

	Ok(())
}

async fn pull_image_from_registry(
	image_name: &str,
	tag: &str,
	config: &Settings,
) -> Result<(), Error> {
	let app = service::get_app().clone();
	let god_username = db::get_user_by_user_id(
		app.database.acquire().await?.deref_mut(),
		rbac::GOD_USER_ID.get().unwrap().as_bytes(),
	)
	.await?
	.status(500)?
	.username;

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

	let docker = Docker::new();
	let mut stream = docker.images().pull(
		&PullOptions::builder()
			.image(format!("{}:{}", &image_name, tag))
			.auth(registry_auth)
			.build(),
	);

	while stream.next().await.is_some() {}

	Ok(())
}

pub async fn app_exists(
	deployment_id: &[u8],
	config: &Settings,
) -> Result<Option<String>, Error> {
	let app = service::get_app().clone();
	let deployment = db::get_deployment_by_id(
		app.database.acquire().await?.deref_mut(),
		deployment_id,
	)
	.await?
	.status(500)
	.body(error!(SERVER_ERROR).to_string())?;

	let app_id = if let Some(app_id) = deployment.digital_ocean_app_id {
		app_id
	} else {
		return Ok(None);
	};

	let deployment_status = Client::new()
		.get(format!("https://api.digitalocean.com/v2/apps/{}", app_id))
		.bearer_auth(&config.digital_ocean_api_key)
		.send()
		.await?
		.status();

	if deployment_status.as_u16() == 404 {
		Ok(None)
	} else if deployment_status.is_success() {
		Ok(Some(app_id))
	} else {
		Err(Error::empty())
	}
}

async fn get_registry_auth_token(config: &Settings) -> Result<String, Error> {
	let registry = Client::new()
		.get("https://api.digitalocean.com/v2/registry/docker-credentials?read_write=true?expiry_seconds=86400")
		.bearer_auth(&config.digital_ocean_api_key)
		.send()
		.await?
		.json::<Auth>()
		.await?;

	Ok(registry.auths.registry.auth)
}

// create a new digital ocean application
pub async fn create_app(
	deployment_id: &[u8],
	tag: &str,
	settings: &Settings,
) -> Result<(), Error> {
	let deploy_app = Client::new()
		.post("https://api.digitalocean.com/v2/apps")
		.bearer_auth(&settings.digital_ocean_api_key)
		.json(&AppConfig {
			spec: {
				AppSpec {
					name: hex::encode(&deployment_id),
					region: "blr".to_string(),
					domains: vec![Domains {
						// [ 4 .. 253 ] characters
						// ^((xn--)?[a-zA-Z0-9]+(-[a-zA-Z0-9]+)*\.)+[a-zA-Z]{2,
						// }\.?$ The hostname for the domain
						domain: format!(
							"{}.vicara.tech",
							hex::encode(deployment_id)
						),
						// for now this has been set to PRIMARY
						r#type: "PRIMARY".to_string(),
					}],
					services: vec![Services {
						name: "default-service".to_string(),
						image: Image {
							registry_type: "DOCR".to_string(),
							repository: hex::encode(deployment_id),
							tag: tag.to_string(),
						},
						// for now instance count is set to 1
						instance_count: 1,
						instance_size_slug: "basic-xs".to_string(),
						http_port: 80,
						routes: vec![Routes {
							path: "/".to_string(),
						}],
					}],
				}
			},
		})
		.send()
		.await?
		.json::<AppHolder>()
		.await?;

	if deploy_app.app.id.is_empty() {
		Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	}

	let app = service::get_app().clone();

	db::update_digital_ocean_app_id_for_deployment(
		app.database.acquire().await?.deref_mut(),
		&deploy_app.app.id,
		deployment_id,
	)
	.await?;

	Ok(())
}

pub async fn redeploy_application(
	app_id: &str,
	config: &Settings,
) -> Result<(), Error> {
	// for now i am not deserializing the response of the request
	// Can be added later if required
	let deployment_info = Client::new()
		.get(format!(
			"https://api.digitalocean.com/v2/apps/{}/deployments",
			app_id
		))
		.bearer_auth(&config.digital_ocean_api_key)
		.send()
		.await?
		.status();

	if deployment_info.is_client_error() || deployment_info.is_server_error() {
		Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	}

	Ok(())
}
