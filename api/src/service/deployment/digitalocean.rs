use std::{ops::DerefMut, process::Stdio, str, time::Duration};

use eve_rs::AsError;
use reqwest::Client;
use tokio::{process::Command, task, time};

use crate::{
	db,
	error,
	models::{
		db_mapping::{
			DeploymentMachineType,
			DeploymentStatus,
			ManagedDatabaseEngine,
			ManagedDatabasePlan,
			ManagedDatabaseStatus,
		},
		deployment::cloud_providers::digitalocean::{
			AppAggregateLogsResponse,
			AppConfig,
			AppDeploymentEnvironmentVariables,
			AppDeploymentsResponse,
			AppHolder,
			AppSpec,
			Auth,
			DatabaseConfig,
			DatabaseResponse,
			Db,
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
	let deployment = db::get_deployment_by_id(
		service::get_app().database.acquire().await?.deref_mut(),
		&deployment_id,
	)
	.await?
	.status(500)
	.body(error!(SERVER_ERROR).to_string())?;

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
		let app_id = create_app(
			&deployment_id,
			region,
			deployment.horizontal_scale,
			&deployment.machine_type,
			&config,
			&client,
		)
		.await?;
		log::trace!("App created");
		app_id
	};

	// wait for the app to be completed to be deployed
	let default_url = wait_for_app_deploy(&app_id, &config, &client).await;
	log::trace!("App ingress is at {}", default_url);

	// update DNS
	log::trace!("updating DNS");
	super::add_cname_record(
		&deployment_id_string,
		"nginx.patr.cloud",
		&config,
		false,
	)
	.await?;
	log::trace!("DNS Updated");

	log::trace!("adding reverse proxy");
	super::update_nginx_with_all_domains_for_deployment(
		&deployment_id_string,
		&default_url,
		deployment.domain_name.as_deref(),
		&config,
	)
	.await?;

	let _ = super::update_deployment_status(
		&deployment_id,
		&DeploymentStatus::Running,
	)
	.await;
	log::trace!("deleting image tagged with digitalocean registry");
	let _ = super::delete_docker_image(&new_repo_name).await?;
	log::trace!("deleting the pulled image");
	let _ = super::delete_docker_image(&image_id).await?;
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
		.digitalocean_app_id;
	let app_id = if let Some(app_id) = app_id {
		log::trace!("deployment ids matched");
		app_id
	} else {
		log::error!("deployment ids did not match");
		return Ok(());
	};

	log::trace!("deleting the image from registry");
	delete_image_from_digitalocean_registry(deployment_id, config).await?;

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
		.map(|deployment| deployment.digitalocean_app_id)
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

pub(super) async fn create_managed_database_cluster(
	connection: &mut <Database as sqlx::Database>::Connection,
	database_id: &[u8],
	db_name: &str,
	engine: &ManagedDatabaseEngine,
	version: &str,
	num_nodes: u64,
	database_plan: &ManagedDatabasePlan,
	region: &str,
	config: &Settings,
) -> Result<(), Error> {
	log::trace!("creating a digital ocean managed database");
	let client = Client::new();

	let do_db_name = format!("database-{}", get_current_time().as_millis());

	let db_engine = match engine {
		ManagedDatabaseEngine::Postgres => "pg",
		ManagedDatabaseEngine::Mysql => "mysql",
	};

	let region = match region {
		"nyc" => "nyc1",
		"ams" => "ams3",
		"sfo" => "sfo3",
		"sgp" => "sgp1",
		"lon" => "lon1",
		"fra" => "fra1",
		"tor" => "tor1",
		"blr" => "blr1",
		"any" => "blr1",
		_ => {
			return Err(Error::empty()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string()))
		}
	};

	log::trace!("sending the create db cluster request to digital ocean");
	let database_cluster = client
		.post("https://api.digitalocean.com/v2/databases")
		.bearer_auth(&config.digital_ocean_api_key)
		.json(&DatabaseConfig {
			name: do_db_name, // should be unique
			engine: db_engine.to_string(),
			version: Some(version.to_string()),
			num_nodes,
			size: database_plan.as_do_plan()?,
			region: region.to_string(),
		})
		.send()
		.await?
		.json::<DatabaseResponse>()
		.await?;
	log::trace!("database created");

	db::update_digitalocean_db_id_for_database(
		connection,
		database_id,
		&database_cluster.database.id,
	)
	.await?;

	let database_id = database_id.to_vec();
	let db_name = db_name.to_string();

	task::spawn(async move {
		let result = update_database_cluster_credentials(
			database_id.clone(),
			db_name,
			database_cluster.database.id,
		)
		.await;

		if let Err(error) = result {
			let _ = super::update_managed_database_status(
				&database_id,
				&ManagedDatabaseStatus::Errored,
			)
			.await;
			log::error!(
				"Error while creating managed database, {}",
				error.get_error()
			);
		}
	});

	Ok(())
}

pub(super) async fn delete_database(
	digitalocean_db_id: &str,
	config: &Settings,
) -> Result<(), Error> {
	let client = Client::new();

	let database_status = client
		.delete(format!(
			"https://api.digitalocean.com/v2/databases/{}",
			digitalocean_db_id
		))
		.bearer_auth(&config.digital_ocean_api_key)
		.send()
		.await?
		.status();

	if database_status.is_client_error() || database_status.is_server_error() {
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	}
	Ok(())
}

pub(super) async fn get_app_default_url(
	deployment_id: &[u8],
	config: &Settings,
	client: &Client,
) -> Result<Option<String>, Error> {
	let app_id = if let Some(app_id) =
		app_exists(deployment_id, config, client).await?
	{
		app_id
	} else {
		return Ok(None);
	};
	Ok(get_app_default_ingress(&app_id, config, client)
		.await
		.map(|ingress| ingress.replace("https://", "").replace("/", "")))
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

	let app_id = if let Some(app_id) = deployment.digitalocean_app_id {
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

async fn update_database_cluster_credentials(
	database_id: Vec<u8>,
	db_name: String,
	digitalocean_db_id: String,
) -> Result<(), Error> {
	let client = Client::new();
	let settings = service::get_settings();

	// Wait for the database to be online
	log::trace!("waiting for database to be online");
	loop {
		let database_status = client
			.get(format!(
				"https://api.digitalocean.com/v2/databases/{}",
				digitalocean_db_id
			))
			.bearer_auth(&settings.digital_ocean_api_key)
			.send()
			.await?
			.json::<DatabaseResponse>()
			.await?;

		if database_status.database.status == "online" {
			super::update_managed_database_credentials_for_database(
				&database_id,
				&database_status.database.connection.host,
				database_status.database.connection.port as i32,
				&database_status.database.connection.user,
				&database_status.database.connection.password,
			)
			.await?;
			super::update_managed_database_status(
				&database_id,
				&ManagedDatabaseStatus::Running,
			)
			.await?;
			break;
		}

		time::sleep(Duration::from_millis(1000)).await;
	}
	log::trace!("database online");

	log::trace!("creating a new database inside cluster");
	let new_db_status = client
		.post(format!(
			"https://api.digitalocean.com/v2/databases/{}/dbs",
			digitalocean_db_id
		))
		.bearer_auth(&settings.digital_ocean_api_key)
		.json(&Db { name: db_name })
		.send()
		.await?
		.status();

	if new_db_status.is_client_error() || new_db_status.is_server_error() {
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	}

	log::trace!("updating to the db status to running");
	// wait for database to start
	super::update_managed_database_status(
		&database_id,
		&ManagedDatabaseStatus::Running,
	)
	.await?;
	log::trace!("database successfully updated");

	Ok(())
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
	horizontal_scale: i16,
	machine_type: &DeploymentMachineType,
	settings: &Settings,
	client: &Client,
) -> Result<String, Error> {
	let envs = db::get_environment_variables_for_deployment(
		service::get_app().database.acquire().await?.deref_mut(),
		deployment_id,
	)
	.await?
	.into_iter()
	.map(|(key, value)| AppDeploymentEnvironmentVariables {
		key,
		value,
		scope: "RUN_AND_BUILD_TIME".to_string(),
		r#type: "GENERAL".to_string(),
	})
	.collect();

	let deploy_app = client
		.post("https://api.digitalocean.com/v2/apps")
		.bearer_auth(&settings.digital_ocean_api_key)
		.json(&AppConfig {
			spec: AppSpec {
				name: format!("deployment-{}", get_current_time().as_millis()),
				region,
				domains: vec![],
				services: vec![Services {
					name: "default-service".to_string(),
					image: Image {
						registry_type: "DOCR".to_string(),
						repository: hex::encode(deployment_id),
						tag: "latest".to_string(),
					},
					// for now instance count is set to 1
					instance_count: horizontal_scale as u64,
					instance_size_slug:
						match (machine_type, horizontal_scale) {
							(DeploymentMachineType::Micro, 1) => "basic-xxs",
							(DeploymentMachineType::Micro, _) => {
								"professional-xs"
							}
							(DeploymentMachineType::Small, 1) => "basic-xs",
							(DeploymentMachineType::Small, _) => {
								"professional-xs"
							}
							(DeploymentMachineType::Medium, 1) => "basic-s",
							(DeploymentMachineType::Medium, _) => {
								"professional-s"
							}
							(DeploymentMachineType::Large, 1) => "basic-m",
							(DeploymentMachineType::Large, _) => {
								"professional-m"
							}
						}
						.to_string(),
					http_port: 80,
					routes: vec![Routes {
						path: "/".to_string(),
					}],
					envs,
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

	db::update_digitalocean_app_id_for_deployment(
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

async fn wait_for_app_deploy(
	app_id: &str,
	config: &Settings,
	client: &Client,
) -> String {
	loop {
		if let Some(ingress) =
			get_app_default_ingress(app_id, config, client).await
		{
			break ingress.replace("https://", "").replace("/", "");
		}
		time::sleep(Duration::from_millis(1000)).await;
	}
}

async fn get_app_default_ingress(
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

async fn delete_image_from_digitalocean_registry(
	deployment_id: &[u8],
	config: &Settings,
) -> Result<(), Error> {
	let client = Client::new();

	let container_status = client
		.delete(format!(
			"https://api.digitalocean.com/v2/registry/patr-cloud/repositories/{}/tags/latest",
			hex::encode(deployment_id)
		))
		.bearer_auth(&config.digital_ocean_api_key)
		.send()
		.await?
		.status();

	if container_status.is_server_error() || container_status.is_client_error()
	{
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string());
	}

	Ok(())
}
