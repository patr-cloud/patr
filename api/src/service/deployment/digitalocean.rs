use std::{ops::DerefMut, process::Stdio, str, time::Duration};

use eve_rs::AsError;
use reqwest::Client;
use tokio::{process::Command, time};

use crate::{
	db,
	error,
	models::{
		db_mapping::DeploymentStatus,
		deployment::cloud_providers::digitalocean::{
			AppAggregateLogsResponse,
			AppConfig,
			AppDeploymentsResponse,
			AppHolder,
			AppSpec,
			Auth,
			Domains,
			Image,
			RedeployAppRequest,
			Routes,
			Services,
		},
	},
	service,
	utils::{get_current_time, settings::Settings, Error},
	Database,
};

pub(super) async fn deploy_container(
	image_id: String,
	region: String,
	deployment_id: Vec<u8>,
	config: Settings,
) -> Result<(), Error> {
	let client = Client::new();
	let deployment_id_string = hex::encode(&deployment_id);
	log::trace!("Deploying deployment: {}", deployment_id_string);
	let _ = super::update_deployment_status(
		&deployment_id,
		&DeploymentStatus::Pushed,
	)
	.await;

	log::trace!("Pulling image from registry");
	super::pull_image_from_registry(&image_id, &config).await?;
	log::trace!("Image pulled");

	// new name for the docker image
	let new_repo_name = format!(
		"registry.digitalocean.com/patr-cloud/{}",
		deployment_id_string
	);
	log::trace!("Pushing to {}", new_repo_name);

	// rename the docker image with the digital ocean registry url
	super::tag_docker_image(&image_id, &new_repo_name).await?;
	log::trace!("Image tagged");

	// Get login details from digital ocean registry and decode from base 64 to
	// binary
	let auth_token =
		base64::decode(get_registry_auth_token(&config, &client).await?)?;
	log::trace!("Got auth token");

	// Convert auth token from binary to utf8
	let auth_token = str::from_utf8(&auth_token)?;
	log::trace!("Decoded auth token as {}", auth_token);

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
	log::trace!("Logged into DO registry");

	if !output.success() {
		return Err(Error::empty()
			.status(500)
			.body(error!(SERVER_ERROR).to_string()));
	}
	log::trace!("Login was success");

	let do_image_name = format!(
		"registry.digitalocean.com/patr-cloud/{}",
		deployment_id_string
	);
	// if the loggin in is successful the push the docker image to registry
	let push_status = Command::new("docker")
		.arg("push")
		.arg(&do_image_name)
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()?
		.wait()
		.await?;
	log::trace!("Pushing to DO to {}", do_image_name);

	if !push_status.success() {
		return Err(Error::empty()
			.status(500)
			.body(error!(SERVER_ERROR).to_string()));
	}
	log::trace!("Pushed to DO");

	// if the app exists then only create a deployment
	let app_exists = app_exists(&deployment_id, &config, &client).await?;
	log::trace!("App exists as {:?}", app_exists);

	let _ = super::update_deployment_status(
		&deployment_id,
		&DeploymentStatus::Deploying,
	)
	.await;

	let app_id = if let Some(app_id) = app_exists {
		// the function to create a new deployment
		redeploy_application(&app_id, &config, &client).await?;
		log::trace!("App redeployed");
		app_id
	} else {
		// if the app doesn't exists then create a new app
		let app_id =
			create_app(&deployment_id, region, &config, &client).await?;
		log::trace!("App created");
		app_id
	};

	// wait for the app to be completed to be deployed
	let default_ingress = wait_for_deploy(&app_id, &config, &client).await;
	log::trace!("App ingress is at {}", default_ingress);

	// update DNS
	log::trace!("updating DNS");
	super::add_cname_record(&deployment_id_string, &default_ingress, &config)
		.await?;
	log::trace!("DNS Updated");

	let _ = super::update_deployment_status(
		&deployment_id,
		&DeploymentStatus::Running,
	)
	.await;
	let _ =
		super::delete_docker_image(&deployment_id_string, &image_id)
			.await;
	log::trace!("Docker image deleted");

	Ok(())
}

pub(super) async fn delete_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	config: &Settings,
) -> Result<(), Error> {
	log::trace!("retreiving and comparing the deployment ids");
	let app_id = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(500)?
		.digital_ocean_app_id;
	let app_id = if let Some(app_id) = app_id {
		log::trace!("deployment ids matched");
		app_id
	} else {
		log::error!("deployment ids did not match");
		return Ok(());
	};
	
	log::trace!("deleting the deployment");
	let response = Client::new()
		.delete(format!("https://api.digitalocean.com/v2/apps/{}", app_id))
		.bearer_auth(&config.digital_ocean_api_key)
		.send()
		.await?
		.status();

	if response.is_success() {
		log::trace!("deployment deleted successfully!");
		Ok(())
	} else {
		log::trace!("deployment deletion failed");
		Err(Error::empty())
	}
}

pub(super) async fn get_container_logs(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	config: &Settings,
) -> Result<String, Error> {
	let client = Client::new();

	log::info!("retreiving deployment info from db");
	let app_id = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.map(|deployment| deployment.digital_ocean_app_id)
		.flatten()
		.status(500)?;

	log::info!("getting app id from digitalocean api");
	let deployment_id = client
		.get(format!(
			"https://api.digitalocean.com/v2/apps/{}/deployments",
			app_id
		))
		.bearer_auth(&config.digital_ocean_api_key)
		.send()
		.await?
		.json::<AppDeploymentsResponse>()
		.await?
		.deployments
		.into_iter()
		.next()
		.map(|deployment| deployment.id)
		.status(500)?;

	log::info!("getting RUN logs from digitalocean");
	let log_url = client
		.get(format!(
			"https://api.digitalocean.com/v2/apps/{}/deployments/{}/logs",
			app_id, deployment_id
		))
		.query(&[("type", "RUN")])
		.bearer_auth(&config.digital_ocean_api_key)
		.send()
		.await?
		.json::<AppAggregateLogsResponse>()
		.await?
		.live_url;

	let logs = client.get(log_url).send().await?.text().await?;
	log::info!("logs retreived successfully!");
	Ok(logs)
}

async fn app_exists(
	deployment_id: &[u8],
	config: &Settings,
	client: &Client,
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

	let deployment_status = client
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

async fn get_registry_auth_token(
	config: &Settings,
	client: &Client,
) -> Result<String, Error> {
	let registry = client
		.get("https://api.digitalocean.com/v2/registry/docker-credentials?read_write=true?expiry_seconds=86400")
		.bearer_auth(&config.digital_ocean_api_key)
		.send()
		.await?
		.json::<Auth>()
		.await?;

	Ok(registry.auths.registry.auth)
}

// create a new digital ocean application
async fn create_app(
	deployment_id: &[u8],
	region: String,
	settings: &Settings,
	client: &Client,
) -> Result<String, Error> {
	let deploy_app = client
		.post("https://api.digitalocean.com/v2/apps")
		.bearer_auth(&settings.digital_ocean_api_key)
		.json(&AppConfig {
			spec: AppSpec {
				name: format!("deployment-{}", get_current_time().as_millis()),
				region,
				domains: vec![Domains {
					// [ 4 .. 253 ] characters
					// ^((xn--)?[a-zA-Z0-9]+(-[a-zA-Z0-9]+)*\.)+[a-zA-Z]{2,
					// }\.?$ The hostname for the domain
					domain: format!(
						"{}.patr.cloud",
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
						tag: "latest".to_string(),
					},
					// for now instance count is set to 1
					instance_count: 1,
					instance_size_slug: "basic-xs".to_string(),
					http_port: 80,
					routes: vec![Routes {
						path: "/".to_string(),
					}],
				}],
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

	Ok(deploy_app.app.id)
}

async fn redeploy_application(
	app_id: &str,
	config: &Settings,
	client: &Client,
) -> Result<(), Error> {
	// for now i am not deserializing the response of the request
	// Can be added later if required
	let status = client
		.post(format!(
			"https://api.digitalocean.com/v2/apps/{}/deployments",
			app_id
		))
		.bearer_auth(&config.digital_ocean_api_key)
		.json(&RedeployAppRequest { force_build: true })
		.send()
		.await?
		.status();

	if !status.is_success() {
		Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	}

	Ok(())
}

async fn wait_for_deploy(
	app_id: &str,
	config: &Settings,
	client: &Client,
) -> String {
	loop {
		if let Some(ingress) = get_default_ingress(app_id, config, client).await
		{
			break ingress.replace("https://", "").replace("/", "");
		}
		time::sleep(Duration::from_millis(1000)).await;
	}
}

async fn get_default_ingress(
	app_id: &str,
	config: &Settings,
	client: &Client,
) -> Option<String> {
	client
		.get(format!("https://api.digitalocean.com/v2/apps/{}", app_id))
		.bearer_auth(&config.digital_ocean_api_key)
		.send()
		.await
		.ok()?
		.json::<AppHolder>()
		.await
		.ok()?
		.app
		.default_ingress
}

pub async fn delete_container_from_cloud_registry(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	config: &Settings,
) -> Result<(), Error> {
	let client = Client::new();
	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let image_name = deployment
		.deployed_image
		.status(404)
		.body(error!(NOT_FOUND).to_string())?;

	let (image_name, digest) = image_name
		.split_once("@")
		.status(404)
		.body(error!(NOT_FOUND).to_string())?;

	let container_status = client
		.delete(format!(
			"https://api.digitalocean.com/v2/registry/patr-cloud/{}/digests/{}",
			image_name, digest
		))
		.bearer_auth(&config.digital_ocean_api_key)
		.send()
		.await?
		.status();

	if container_status.is_server_error() || container_status.is_client_error()
	{
		return Error::as_result()
			.status(500)
			.body(error!(WRONG_PARAMETERS).to_string());
	}

	let garbage_status = client
		.post("https://api.digitalocean.com/v2/registry/patr-cloud/garbage-collection")
		.bearer_auth(&config.digital_ocean_api_key)
		.send()
		.await?
		.status();

	if garbage_status.is_server_error() || container_status.is_client_error() {
		return Error::as_result()
			.status(500)
			.body(error!(WRONG_PARAMETERS).to_string());
	}

	Ok(())
}