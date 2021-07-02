use std::{collections::HashSet, net::IpAddr, ops::DerefMut, time::Duration};

use futures::StreamExt;
use openssh::*;
use shiplift::{builder::ContainerListOptionsBuilder, ContainerFilter, Docker};
use sqlx::{types::ipnetwork::IpNetwork, Pool};
use tokio::{
	sync::{Mutex, RwLock},
	task,
	time,
};
use uuid::Uuid;

use crate::{
	db,
	models::db_mapping::{Deployment, DeploymentApplicationServer},
	service,
	utils::{get_current_time_millis, settings::Settings, Error},
	Database,
};

lazy_static::lazy_static! {
	static ref DEPLOYMENTS: Mutex<HashSet<Vec<u8>>> = Mutex::new(HashSet::new());
	static ref SHOULD_EXIT: RwLock<bool> = RwLock::new(false);
}

pub async fn monitor_all_deployments() {
	let app = service::get_app().clone();

	task::spawn(async {
		tokio::signal::ctrl_c()
			.await
			.expect("unable to listen for exit event");
		*SHOULD_EXIT.write().await = true;
	});

	// Register runner
	let runner_id = loop {
		break match register_runner(&app.database).await {
			Ok(value) => value.as_bytes().to_vec(),
			Err(error) => {
				log::error!("Error registering runner: {}", error.get_error());
				time::sleep(Duration::from_millis(500)).await;
				continue;
			}
		};
	};
	log::info!("Registered with runnerId `{}`", hex::encode(&runner_id));

	// Register all application servers
	while let Err(error) =
		register_application_servers(&app.database, &app.config).await
	{
		if *SHOULD_EXIT.read().await {
			if let Err(error) =
				unset_container_id_for_runner(&app.database, &runner_id).await
			{
				log::error!(
					"Error unsetting container_id: {}",
					error.get_error()
				);
				time::sleep(Duration::from_secs(10)).await;
			}
			return;
		}
		log::error!("Error registering servers: {}", error.get_error());
		time::sleep(Duration::from_millis(1000)).await;
	}

	// Continously monitor deployments
	loop {
		if *SHOULD_EXIT.read().await {
			// should exit. Wait for all runners to stop and quit

			// Set container_id of runner to null first.
			// If that fails, wait for at least 10 seconds so that the
			// last_updated is invalidated
			if let Err(error) =
				unset_container_id_for_runner(&app.database, &runner_id).await
			{
				log::error!(
					"Error unsetting container_id: {}",
					error.get_error()
				);
				time::sleep(Duration::from_secs(10)).await;
			}

			while !DEPLOYMENTS.lock().await.is_empty() {
				// Wait for 1 second and try again
				time::sleep(Duration::from_millis(1000)).await;
			}
			break;
		}

		// If the runner is supposed to be running, check for deployments to
		// run, run them and regularly update the runner status
		if let Ok(deployments) = get_deployments_to_run(&app.database).await {
			for deployment in deployments {
				task::spawn(monitor_deployment(
					app.database.clone(),
					runner_id.clone(),
					deployment,
					app.config.clone(),
				));
			}
		} else {
			if let Err(error) =
				update_runner_status(&app.database, &runner_id).await
			{
				log::error!(
					"Error updating runner status: {}",
					error.get_error()
				);
			}
			time::sleep(Duration::from_millis(1000)).await;
			continue;
		}
		// Every 2.5 seconds, update the runner status. 1 minute later, recheck
		// for deployments again
		for _ in 0..=24 {
			if let Err(error) =
				update_runner_status(&app.database, &runner_id).await
			{
				log::error!(
					"Error updating runner status: {}",
					error.get_error()
				);
			}
			if *SHOULD_EXIT.read().await {
				break;
			}
			time::sleep(Duration::from_millis(2500)).await;
		}
	}
}

async fn register_runner(pool: &Pool<Database>) -> Result<Uuid, Error> {
	let mut container_id;

	loop {
		if *SHOULD_EXIT.read().await {
			return Err(Error::empty());
		}
		container_id =
			db::generate_new_container_id(pool.acquire().await?.deref_mut())
				.await?;
		let outdated_runner = db::get_inoperative_deployment_runner(
			pool.acquire().await?.deref_mut(),
		)
		.await?;
		let runner = if let Some(runner) = outdated_runner {
			runner
		} else {
			db::register_new_deployment_runner(
				pool.acquire().await?.deref_mut(),
				container_id.as_bytes(),
			)
			.await?;
			break;
		};

		db::update_deployment_runner_container_id(
			pool.acquire().await?.deref_mut(),
			&runner.id,
			Some(container_id.as_bytes()),
			runner.container_id.as_deref(),
		)
		.await?;
		let runner = if let Some(runner) = db::get_deployment_runner_by_id(
			pool.acquire().await?.deref_mut(),
			&runner.id,
		)
		.await?
		{
			runner
		} else {
			continue;
		};

		if runner.container_id.as_deref() != Some(container_id.as_bytes()) {
			time::sleep(Duration::from_millis(100)).await;
			continue;
		}

		db::update_deployment_runner_container_id(
			pool.acquire().await?.deref_mut(),
			&runner.id,
			Some(runner.id.as_ref()),
			runner.container_id.as_deref(),
		)
		.await?;
		container_id = Uuid::from_slice(&runner.id)?;
		break;
	}

	Ok(container_id)
}

#[cfg(not(debug_assertions))]
async fn get_servers_from_cloud_provider(
	settings: &Settings,
) -> Result<Vec<IpAddr>, Error> {
	use reqwest::Client;

	use crate::models::deployment::cloud_providers::digital_ocean::DropletDetails;

	let private_ipv4_address = Client::new()
		.get("https://api.digitalocean.com/v2/droplets?per_page=200")
		.bearer_auth(&settings.digital_ocean_api_key)
		.send()
		.await?
		.json::<Vec<DropletDetails>>()
		.await?
		.into_iter()
		.filter_map(|droplet| {
			droplet.networks.v4.into_iter().find_map(|ipv4| {
				if ipv4.r#type == "private" {
					Some(IpAddr::V4(ipv4.ip_address))
				} else {
					None
				}
			})
		})
		.collect();

	Ok(private_ipv4_address)
}

#[cfg(debug_assertions)]
async fn get_servers_from_cloud_provider(
	_settings: &Settings,
) -> Result<Vec<IpAddr>, Error> {
	use std::net::Ipv4Addr;

	Ok(vec![IpAddr::V4(Ipv4Addr::LOCALHOST)])
}

async fn register_application_servers(
	pool: &Pool<Database>,
	settings: &Settings,
) -> Result<(), Error> {
	// Check to make sure servers are registered correctly
	let servers = get_servers_from_cloud_provider(settings).await?;

	let mut connection = pool.begin().await?;

	for (index, server) in servers.iter().enumerate() {
		db::register_deployment_application_server(&mut connection, server)
			.await?;

		// Every 10th iteration, check if it should exit, to prevent
		// unnecessarry showdown of the application
		if index % 10 == 0 && *SHOULD_EXIT.read().await {
			return Err(Error::empty());
		}
	}
	db::remove_excess_deployment_application_servers(
		&mut connection,
		servers
			.into_iter()
			.map(IpNetwork::from)
			.collect::<Vec<_>>()
			.as_ref(),
	)
	.await?;

	connection.commit().await?;
	Ok(())
}

async fn get_deployments_to_run(
	pool: &Pool<Database>,
) -> Result<Vec<Deployment>, Error> {
	let deployments = db::get_deployments_not_running_for_runner(
		&mut pool.begin().await?.deref_mut(),
	)
	.await?;

	Ok(deployments)
}

async fn update_runner_status(
	pool: &Pool<Database>,
	runner_id: &[u8],
) -> Result<(), Error> {
	db::update_deployment_runner_last_updated(
		pool.acquire().await?.deref_mut(),
		runner_id,
		get_current_time_millis(),
	)
	.await?;

	Ok(())
}

async fn unset_container_id_for_runner(
	pool: &Pool<Database>,
	runner_id: &[u8],
) -> Result<(), Error> {
	log::error!("Unsetting for {}", hex::encode(runner_id));
	db::update_deployment_runner_container_id(
		pool.acquire().await?.deref_mut(),
		runner_id,
		None,
		Some(runner_id),
	)
	.await?;

	Ok(())
}

async fn monitor_deployment(
	pool: Pool<Database>,
	runner_id: Vec<u8>,
	deployment: Deployment,
	settings: Settings,
) {
	let mut deployments = DEPLOYMENTS.lock().await;
	if deployments.contains(&deployment.id) {
		// Some other task is already running this deployment.
		// Exit and let the other task handle it
		return;
	}
	deployments.insert(deployment.id.clone());
	drop(deployments);

	let mut old_deployed_server: Option<DeploymentApplicationServer> = None;

	loop {
		if *SHOULD_EXIT.read().await {
			break;
		}
		// First, find available server to deploy to
		let server = loop {
			break match get_available_server_for_deployment(&pool, 1, 1).await {
				Ok(Some(server)) => server,
				Ok(None) => {
					match create_new_application_server(&settings).await {
						Ok(server) => server,
						Err(error) => {
							log::error!(
								"Error creating new application server: {}",
								error.get_error()
							);
							time::sleep(Duration::from_millis(5000)).await;
							continue;
						}
					}
				}
				Err(error) => {
					log::error!(
						"Error getting servers for deployment: {}",
						error.get_error()
					);
					time::sleep(Duration::from_millis(500)).await;
					continue;
				}
			};
		};

		// now that there's a server available, open a reverse tunnel, get
		// the docker socket, and run the image on it.
		let server_ip = server.server_ip.ip();
		let mut error_count = 0;
		let ssh_result = loop {
			if error_count >= 5 {
				// 5 attempts to connect to the server has failed.
				// TODO Mark the server as inactive and find another server
				break Err(Error::empty());
			}
			break match Session::connect(
				format!("ssh://deployment@{}", server_ip),
				KnownHosts::Add,
			)
			.await
			{
				Ok(session) => Ok(session),
				Err(error) => {
					error_count += 1;
					log::error!(
						"Unable to connect to server `{}`: {}",
						server_ip,
						error
					);
					time::sleep(Duration::from_millis(1000)).await;
					continue;
				}
			};
		};
		let _ssh_connection = if let Ok(session) = ssh_result {
			session
		} else {
			continue;
		};
		// TODO open reverse tunnel using `_ssh_connection` here
		let docker = Docker::unix(format!(
			"./application-server-{}-docker.sock",
			server_ip
		));

		// Monitor deployment here
		loop {
			let stats = if let Some(Ok(stats)) = docker
				.containers()
				.get(format!("deployment-{}", hex::encode(&deployment.id)))
				.stats()
				.next()
				.await
			{
				stats
			} else {
				continue;
			};
		}
	}
	DEPLOYMENTS.lock().await.remove(&deployment.id);
}

async fn get_available_server_for_deployment(
	pool: &Pool<Database>,
	memory_requirement: u16,
	cpu_requirement: u8,
) -> Result<Option<DeploymentApplicationServer>, Error> {
	let server = db::get_available_deployment_server_for_deployment(
		pool.acquire().await?.deref_mut(),
		memory_requirement,
		cpu_requirement,
	)
	.await?;

	Ok(server)
}

#[cfg(not(debug_assertions))]
async fn create_new_application_server(
	settings: &Settings,
) -> Result<DeploymentApplicationServer, Error> {
	// TODO create server using DO APIs
	Err(Error::empty())
}

#[cfg(debug_assertions)]
async fn create_new_application_server(
	_settings: &Settings,
) -> Result<DeploymentApplicationServer, Error> {
	use std::net::Ipv4Addr;

	use sqlx::types::ipnetwork::Ipv4Network;

	Ok(DeploymentApplicationServer {
		server_ip: IpNetwork::V4(Ipv4Network::from(Ipv4Addr::LOCALHOST)),
		server_type: String::from("default"),
	})
}
