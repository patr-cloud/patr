use std::{ops::DerefMut, process::Stdio, str, time::Duration};

use cloudflare::{
	endpoints::{
		dns::{
			CreateDnsRecord,
			CreateDnsRecordParams,
			DnsContent,
			ListDnsRecords,
			ListDnsRecordsParams,
			UpdateDnsRecord,
			UpdateDnsRecordParams,
		},
		zone::{ListZones, ListZonesParams},
	},
	framework::{
		async_api::{ApiClient, Client as CloudflareClient},
		auth::Credentials,
		Environment,
		HttpApiClientConfig,
	},
};
use eve_rs::AsError;
use futures::StreamExt;
use reqwest::Client;
use shiplift::{Docker, PullOptions, RegistryAuth, TagOptions};
use tokio::{process::Command, time};

use crate::{
	db,
	error,
	models::{
		db_mapping::DeploymentStatus,
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
	Database,
};

pub async fn deploy_container_on_digitalocean(
	image_name: String,
	tag: String,
	deployment_id: Vec<u8>,
	config: Settings,
) -> Result<(), Error> {
	let client = Client::new();
	let deployment_id_string = hex::encode(&deployment_id);

	let _ = update_deployment_status(&deployment_id, &DeploymentStatus::Pushed)
		.await;

	pull_image_from_registry(&image_name, &tag, &config).await?;

	// new name for the docker image
	let new_repo_name = format!(
		"registry.digitalocean.com/patr-cloud/{}",
		deployment_id_string
	);

	// rename the docker image with the digital ocean registry url
	tag_docker_image(&image_name, &new_repo_name).await?;

	// Get login details from digital ocean registry and decode from base 64 to
	// binary
	let auth_token =
		base64::decode(get_registry_auth_token(&config, &client).await?)?;

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
			"registry.digitalocean.com/patr-cloud/{}",
			deployment_id_string
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
	let app_exists = app_exists(&deployment_id, &config, &client).await?;

	let _ =
		update_deployment_status(&deployment_id, &DeploymentStatus::Deploying)
			.await;

	if let Some(app_id) = app_exists {
		// the function to create a new deployment
		redeploy_application(&app_id, &config, &client).await?;
	} else {
		// if the app doesn't exists then create a new app
		let app_id = create_app(&deployment_id, &tag, &config, &client).await?;

		// wait for the app to be completed to be deployed
		let default_ingress = wait_for_deploy(&app_id, &config, &client).await;

		// update DNS
		update_dns(&deployment_id_string, &default_ingress, &config).await?;
	}

	let _ =
		update_deployment_status(&deployment_id, &DeploymentStatus::Running)
			.await;
	let _ = delete_docker_image(&deployment_id_string, &image_name, &tag).await;

	Ok(())
}

pub async fn delete_deployment_from_digital_ocean(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	config: &Settings,
) -> Result<(), Error> {
	let app_id = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(500)?
		.digital_ocean_app_id;
	let app_id = if let Some(app_id) = app_id {
		app_id
	} else {
		return Ok(());
	};

	let response = Client::new()
		.delete(format!("https://api.digitalocean.com/v2/apps/{}", app_id))
		.bearer_auth(&config.digital_ocean_api_key)
		.send()
		.await?
		.status();

	if response.is_success() {
		Ok(())
	} else {
		Err(Error::empty())
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
pub async fn create_app(
	deployment_id: &[u8],
	tag: &str,
	settings: &Settings,
	client: &Client,
) -> Result<String, Error> {
	let deploy_app = client
		.post("https://api.digitalocean.com/v2/apps")
		.bearer_auth(&settings.digital_ocean_api_key)
		.json(&AppConfig {
			spec: {
				AppSpec {
					name: format!(
						"deployment-{}",
						get_current_time().as_millis()
					),
					region: "blr".to_string(),
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

	Ok(deploy_app.app.id)
}

pub async fn redeploy_application(
	app_id: &str,
	config: &Settings,
	client: &Client,
) -> Result<(), Error> {
	// for now i am not deserializing the response of the request
	// Can be added later if required
	let deployment_info = client
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

async fn update_dns(
	sub_domain: &str,
	default_ingress: &str,
	config: &Settings,
) -> Result<(), Error> {
	let full_domain = format!("{}.patr.cloud", sub_domain);
	let credentials = Credentials::UserAuthToken {
		token: config.cloudflare.api_token.clone(),
	};
	let client = if let Ok(client) = CloudflareClient::new(
		credentials,
		HttpApiClientConfig::default(),
		Environment::Production,
	) {
		client
	} else {
		return Err(Error::empty());
	};

	let zone_identifier = client
		.request(&ListZones {
			params: ListZonesParams {
				name: Some(String::from("patr.cloud")),
				..Default::default()
			},
		})
		.await?
		.result
		.into_iter()
		.next()
		.status(500)?
		.id;
	let zone_identifier = zone_identifier.as_str();

	let expected_dns_record = DnsContent::CNAME {
		content: String::from(default_ingress),
	};

	let response = client
		.request(&ListDnsRecords {
			zone_identifier,
			params: ListDnsRecordsParams {
				name: Some(full_domain.clone()),
				..Default::default()
			},
		})
		.await?;
	let dns_record = response.result.into_iter().find(|record| {
		if let DnsContent::CNAME { .. } = record.content {
			record.name == full_domain
		} else {
			false
		}
	});

	if let Some(record) = dns_record {
		if let DnsContent::CNAME { content } = record.content {
			if content != default_ingress {
				client
					.request(&UpdateDnsRecord {
						zone_identifier,
						identifier: record.id.as_str(),
						params: UpdateDnsRecordParams {
							content: expected_dns_record,
							name: &full_domain,
							proxied: Some(true),
							ttl: Some(1),
						},
					})
					.await?;
			}
		}
	} else {
		// Create
		client
			.request(&CreateDnsRecord {
				zone_identifier,
				params: CreateDnsRecordParams {
					content: expected_dns_record,
					name: sub_domain,
					ttl: Some(1),
					priority: None,
					proxied: Some(true),
				},
			})
			.await?;
	}
	Ok(())
}

async fn delete_docker_image(
	deployment_id_string: &str,
	image_name: &str,
	tag: &str,
) -> Result<(), Error> {
	let docker = Docker::new();

	docker
		.images()
		.get(format!(
			"registry.digitalocean.com/patr-cloud/{}:latest",
			deployment_id_string
		))
		.delete()
		.await?;

	docker
		.images()
		.get(format!("{}:{}", image_name, tag))
		.delete()
		.await?;

	Ok(())
}

async fn update_deployment_status(
	deployment_id: &[u8],
	status: &DeploymentStatus,
) -> Result<(), Error> {
	let app = service::get_app();

	db::update_deployment_status(
		app.database.acquire().await?.deref_mut(),
		deployment_id,
		status,
	)
	.await?;

	Ok(())
}
