use std::{collections::HashSet, net::IpAddr, time::Duration};

use sqlx::{types::ipnetwork::IpNetwork, Pool};
use tokio::{
	sync::{Mutex, RwLock},
	task,
	time,
};
use uuid::Uuid;

use crate::{
	db,
	service,
	utils::{settings::Settings, Error},
	Database,
};

lazy_static::lazy_static! {
	static ref DEPLOYMENTS: Mutex<HashSet<Vec<u8>>> = Mutex::new(HashSet::new());
	static ref SHOULD_EXIT: RwLock<bool> = RwLock::new(false);
}

pub async fn monitor_deployments() {
	let app = service::get_app().clone();
	task::spawn(async {
		tokio::signal::ctrl_c()
			.await
			.expect("unable to listen for exit event");
		*SHOULD_EXIT.write().await = true;
	});

	// Register runner
	// let runner_id;
	// loop {
	// 	match register_runner(&app.database).await {
	// 		Ok(value) => {
	// 			runner_id = value;
	// 			break;
	// 		}
	// 		Err(error) => {
	// 			log::error!("Error registering runner: {}", error.get_error());
	// 			time::sleep(Duration::from_millis(500)).await;
	// 		}
	// 	}
	// }

	// Register all application servers
	loop {
		if let Err(error) =
			register_application_servers(&app.database, &app.config).await
		{
			log::error!("Error registering servers: {}", error.get_error());
			time::sleep(Duration::from_millis(1000)).await;
		} else {
			break;
		}
	}

	// Continously monitor deployments
	loop {}
}

async fn register_runner(pool: &Pool<Database>) -> Result<Uuid, Error> {
	let mut connection = pool.begin().await?;
	let mut container_id;

	loop {
		container_id = db::generate_new_container_id(&mut connection).await?;
		let outdated_runner =
			db::get_inoperative_deployment_runner(&mut connection).await?;
		if let Some(runner) = outdated_runner {
			db::update_deployment_runner_container_id(
				&mut connection,
				&runner.id,
				Some(container_id.as_bytes()),
				runner.container_id.as_deref(),
			)
			.await?;
			if let Some(runner) =
				db::get_deployment_runner_by_id(&mut connection, &runner.id)
					.await?
			{
				if runner.container_id.as_deref() ==
					Some(container_id.as_bytes())
				{
					db::update_deployment_runner_container_id(
						&mut connection,
						&runner.id,
						Some(runner.id.as_ref()),
						runner.container_id.as_deref(),
					)
					.await?;
					break;
				} else {
					continue;
				}
			} else {
				continue;
			}
		} else {
			db::register_new_deployment_runner(
				&mut connection,
				container_id.as_bytes(),
			)
			.await?;
			break;
		}
	}
	connection.commit().await?;

	Ok(container_id)
}

#[cfg(debug_assertions)]
async fn get_servers_from_cloud_provider(
	settings: &Settings,
) -> Result<Vec<IpAddr>, Error> {
	use reqwest::{header, Client};

	use crate::models::deployment::cloud_providers::digital_ocean::DropletDetails;

	let mut headers = header::HeaderMap::new();

	headers.insert("Content-Type", "application/json".parse().unwrap());
	headers.insert(
		"Authorization",
		format!("Bearer {}", settings.digital_ocean_api_key)
			.parse()
			.unwrap(),
	);

	let private_ipv4_address = Client::new()
		.get("https://api.digitalocean.com/v2/droplets?per_page=200")
		.headers(headers)
		.send()
		.await?
		.json::<Vec<DropletDetails>>()
		.await?
		.into_iter()
		.filter_map(|droplet| {
			droplet
				.networks
				.v4
				.into_iter()
				.find(|ipv4| ipv4.ip_address.is_private())
		})
		.map(|ip_add| IpAddr::V4(ip_add.ip_address))
		.collect();

	Ok(private_ipv4_address)
}

#[cfg(not(debug_assertions))]
async fn get_servers_from_cloud_provider(
	_settings: &Settings,
) -> Result<Vec<IpAddr>, Error> {
	// TODO call digital ocean API here
	Ok(vec![IpAddr::V4(Ipv4Addr::LOCALHOST)])
}

async fn register_application_servers(
	pool: &Pool<Database>,
	settings: &Settings,
) -> Result<(), Error> {
	// Check to make sure servers are registered correctly
	let servers = get_servers_from_cloud_provider(settings).await?;

	let mut connection = pool.begin().await?;

	for server in servers.iter() {
		db::register_deployment_application_server(&mut connection, server)
			.await?;
	}
	db::remove_excess_deployment_application_servers(
		&mut connection,
		servers
			.into_iter()
			.map(|server| IpNetwork::from(server))
			.collect::<Vec<_>>()
			.as_ref(),
	)
	.await?;

	connection.commit().await?;
	Ok(())
}

// async fn run_deployment(
// 	connection: &mut <Database as sqlx::Database>::Connection,
// ) -> Result<(), Error> {
// 	let container_id = db::generate_new_container_id(&mut *connection).await?;
// 	let inoperative_runner =
// 		db::get_list_of_inoperative_runners(connection).await?;

// 	let mut runner;

// 	if inoperative_runner.is_empty() {
// 		if let Some(operative_runner) =
// 			db::get_runner_with_least_deployments(connection).await?
// 		{
// 			runner = operative_runner;
// 		} else {
// 			runner = db::create_new_runner(connection).await?;
// 		}
// 	} else {
// 		runner = inoperative_runner[0];
// 	}
// 	// Not sure how this step can be executed here If the container ID was set
// 	// to the randomly generated value, assume the lock was acquired
// 	// successfully. Else, try again and acquire another runner’s lock.
// 	db::add_container_for_runner();

// 	loop {
// 		// TODO: Store list of servers available from the cloud provider.
// 		// In case any servers are not present in the DB, add them
// 		// If any extra servers are in the DB, remove them.

// 		// TODO: make enum for status
// 		let running_deployments =
// 			db::get_list_of_deployments_from_deployment_runner_with_status(
// 				connection, "runnning",
// 			)
// 			.await?;
// 		let pseudo_running_deployments =
// 			db::get_list_of_deployments_from_deployment(connection).await?;
// 		let dead_deployments =
// 			db::get_list_of_deployments_from_deployment_runner_with_status(
// 				connection, "stopped",
// 			)
// 			.await?;

// 		let faulty_deployments =
// 			pseudo_running_deployments.intersect(dead_deployments);

// 		start_task_for_deployments(connection, faulty_deployments).await?;

// 		//TODO: graceful shutdown
// 	}

// 	// let app = service::get_app().clone();
// 	// let mut should_exit = task::spawn(tokio::signal::ctrl_c());
// 	// let container_id = hex::encode(Uuid::new_v4().as_bytes());
// 	// loop {
// 	// 	// Db stores the PRIMARY KEY(IP address, region) as server details
// 	// 	// Each server has it's own "alive" status independent of the runner's
// 	// 	// "alive" status. If a server's "alive" status isn't updated in a long
// 	// 	// time, any runners in the same region can take over

// 	// 	// Find the list of servers running on the cloud provider
// 	// 	// Find the list of servers registered with the db
// 	// 	// Register any new server and unregister any old server

// 	// 	// Find the list of deployments to deploy.
// 	// 	// Then start deployment monitor for each of them
// 	// 	// Then wait for, idk like 1 minute
// 	// 	let result = start_all_deployment_monitors(app.database.clone()).await;
// 	// 	if let Err(error) = result {
// 	// 		log::error!(
// 	// 			"Unable to start deployment monitors: {}",
// 	// 			error.get_error()
// 	// 		);
// 	// 	}
// 	// 	for _ in 0..60 {
// 	// 		time::sleep(Duration::from_secs(1)).await;
// 	// 		if (&mut should_exit).now_or_never().is_some() {
// 	// 			return;
// 	// 		}
// 	// 	}
// 	// }
// 	Ok(())
// }

// async fn start_task_for_deployments(
// 	connection: &mut <Database as sqlx::Database>::Connection,
// 	faulty_deployments: Deployment,
// ) -> Result<(), Error> {
// 	for image in faulty_deployments {
// 		let server_list =
// 			db::get_suitable_servers_list(connection, image).await?;
// 		set_docker_limits_for_container(connection, image).await?;
// 		loop {
// 			// TODO: figure out a way to determine system usage
// 			let system_usage = get_system_usage().await?;
// 			if system_usage.overall_usage >= image.usage_limit {
// 				if server_limits_crossed(system_usage.overall_usage) {
// 					find_servers_with_usage(image)
// 				}
// 			}
// 			sleep(10);
// 		}
// 	}

// 	Ok(())
// }

// async fn start_all_deployment_monitors(
// 	pool: Pool<Database>,
// ) -> Result<(), Error> {
// 	let mut connection = pool.acquire().await?;

// 	let deployments = db::get_all_deployments(&mut connection).await?;
// 	// TODO change this to "get all deployments that are not already running",
// 	// which will get deployments that haven't been updated in the last 10
// 	// seconds.
// 	let running_deployments = DEPLOYMENTS.lock().await;
// 	for deployment in deployments {
// 		if !running_deployments.contains(&deployment.id) {
// 			task::spawn(monitor_deployment(pool.clone(), deployment.id));
// 		}
// 	}
// 	drop(running_deployments);

// 	Ok(())
// }

// async fn monitor_deployment(pool: Pool<Database>, deployment_id: Vec<u8>) {
// 	loop {
// 		// Monitor deployment every 10 seconds
// 		time::sleep(Duration::from_secs(10)).await;

// 		let mut connection = match pool.acquire().await {
// 			Ok(connection) => connection,
// 			Err(error) => {
// 				log::error!("Unable to aquire db connection: {}", error);
// 				continue;
// 			}
// 		};
// 		let result =
// 			db::get_deployment_by_id(&mut connection, &deployment_id).await;
// 		let deployment = match result {
// 			Ok(deployment) => deployment,
// 			Err(error) => {
// 				log::error!("Unable to get deployment: {}", error);
// 				continue;
// 			}
// 		};
// 		if let Some(deployment) = deployment {
// 			// TODO check if the `deployment` is doing okay
// 		} else {
// 			// If the deployment doesn't exist in the DB, stop monitoring
// 			log::info!("Deployment `{}` doesn't exist in the database anymore. Will stop
// monitoring it.", 				hex::encode(&deployment_id));
// 			break;
// 		}
// 		drop(connection);
// 	}
// 	let mut deployments = DEPLOYMENTS.lock().await;
// 	deployments.remove(&deployment_id);
// 	drop(deployments);
// }
