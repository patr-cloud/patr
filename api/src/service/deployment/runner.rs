use std::{
	collections::HashSet,
	future::Future,
	io::ErrorKind,
	net::IpAddr,
	ops::DerefMut,
	process::Stdio,
	time::Duration,
};

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
		async_api::{ApiClient, Client},
		auth::Credentials,
		Environment,
		HttpApiClientConfig,
	},
};
use eve_rs::AsError;
use futures::StreamExt;
use openssh::*;
use rand::Rng;
use shiplift::{
	rep::ContainerDetails,
	ContainerOptions,
	Docker,
	Error as ShipliftError,
	PullOptions,
	RegistryAuth,
};
use sqlx::{types::ipnetwork::IpNetwork, Pool};
use tokio::{
	fs,
	io::AsyncWriteExt,
	net::{TcpStream, ToSocketAddrs},
	process::Command,
	sync::{Mutex, RwLock},
	task,
	time,
};
use uuid::Uuid;

use crate::{
	db::{self, add_running_deployment_details},
	models::{
		db_mapping::{
			Deployment,
			DeploymentApplicationServer,
			DeploymentRunnerDeployment,
			DeploymentStatus,
		},
		rbac,
		RegistryToken,
		RegistryTokenAccess,
	},
	service,
	utils::{
		get_current_time,
		get_current_time_millis,
		settings::Settings,
		Error,
	},
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

	use crate::models::deployment::cloud_providers::digital_ocean::DropletResponse;

	let private_ipv4_address = Client::new()
		.get("https://api.digitalocean.com/v2/droplets?per_page=200&tag_name=application-server")
		.bearer_auth(&settings.digital_ocean_api_key)
		.send()
		.await?
		.json::<DropletResponse>()
		.await?
		.droplets
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
		db::register_deployment_application_server(
			&mut connection,
			server,
			"default",
		)
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
	mut deployment: Deployment,
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

	// Register deployment to be running on this runner
	let runner = if let Ok(runner) =
		get_registered_runner_for_deployment(&pool, &runner_id, &deployment)
			.await
	{
		runner
	} else {
		DEPLOYMENTS.lock().await.remove(&deployment.id);
		return;
	};

	let mut first_run = true;

	while !*SHOULD_EXIT.read().await {
		let server_ip = match (first_run, &runner) {
			// If the first_run is true AND if there's an existing IP
			(true, Some(runner)) => {
				first_run = false;
				runner.current_server.ip()
			}
			// If either first_run is false OR if there's no runner
			_ => {
				// Check to make sure some other runner hasn't taken over this
				// deployment
				let is_runner_managing_deployment = if let Ok(value) =
					retry_with_delay_or_exit(
						|| {
							is_deployment_managed_by_runner(
								&pool,
								&deployment.id,
								&runner_id,
							)
						},
						250,
						"Error getting running deployment details",
					)
					.await
				{
					value
				} else {
					break;
				};
				if !first_run && !is_runner_managing_deployment {
					break;
				}

				// First, find available server to deploy to
				let server = if let Ok(server) = retry_with_delay_or_exit(
					|| get_available_server_for_deployment(&pool, 1, 1),
					500,
					"Error getting servers for deployment",
				)
				.await
				{
					server
				} else {
					break;
				};

				// If there is not available server to deploy to, create one
				let server = if let Some(server) = server {
					server
				} else {
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
				};
				let server_ip = server.server_ip.ip();

				// If this is the first run, insert into the DB the details
				// of the running server. If this is not the first run, just
				// update the server IP only.
				if first_run {
					if retry_with_delay_or_fail(
						|| async {
							add_running_deployment_details(
								pool.acquire().await?.deref_mut(),
								&deployment.id,
								&runner_id,
								get_current_time_millis(),
								&IpNetwork::from(server_ip),
								&DeploymentStatus::Alive,
							)
							.await?;
							Ok(())
						},
						5,
						500,
						"Error inserting deployment runner details",
					)
					.await
					.is_err()
					{
						break;
					}
				} else if retry_with_delay_or_exit(
					|| async {
						db::update_running_deployment_server_ip(
							pool.acquire().await?.deref_mut(),
							&IpNetwork::from(server_ip),
							&deployment.id,
							&runner_id,
						)
						.await?;

						Ok(())
					},
					250,
					"Error updating current server IP",
				)
				.await
				.is_err()
				{
					break;
				}
				server_ip
			}
		};

		if let Err(error) = run_deployment_on_application_server(
			&pool,
			&runner_id,
			&mut deployment,
			&server_ip,
			&settings,
		)
		.await
		{
			// If there's been an error, re-run it
			log::error!(
				"Error running deployment `{}` on server `{}`: {}. {}",
				hex::encode(&deployment.id),
				server_ip,
				error.get_error(),
				"Will try again on another server"
			);
			continue;
		} else {
			// If there's no error, then it's a graceful exit
			break;
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

async fn get_registered_runner_for_deployment(
	pool: &Pool<Database>,
	runner_id: &[u8],
	deployment: &Deployment,
) -> Result<Option<DeploymentRunnerDeployment>, Error> {
	let runner = retry_with_delay_or_exit(
		|| async {
			let runner = db::get_running_deployment_details_by_id(
				pool.acquire().await?.deref_mut(),
				&deployment.id,
			)
			.await?;

			Ok(runner)
		},
		250,
		"Unable to update database of deployment runner",
	)
	.await?;

	if let Some(deployment_runner) = runner {
		// There's already a runner trying to run this deployment
		// If it's the current runner, make sure you have the lock on
		// `DEPLOYMENTS` and proceed
		if deployment_runner.runner_id == runner_id {
			if !DEPLOYMENTS.lock().await.contains(&deployment.id) {
				// If you don't have the lock, there's something wrong. Just
				// exit. Relaunching the runner tasks will take care of it.
				Err(Error::empty())
			} else {
				Ok(Some(deployment_runner))
			}
		} else {
			// There's some other runner trying to access it. Check if it's
			// outdated. If it is, take over.
			if deployment_runner.last_updated >=
				get_current_time_millis() - (1000 * 10)
			{
				// Last updated was within the last 10 seconds. Not outdated
				Err(Error::empty())
			} else {
				retry_with_delay_or_exit(
					|| async {
						db::update_running_deployment_runner(
							pool.acquire().await?.deref_mut(),
							runner_id,
							&deployment_runner.runner_id,
						)
						.await?;

						Ok(())
					},
					250,
					format!(
						"Error updating runnerId of deployment `{}`",
						hex::encode(&deployment.id)
					),
				)
				.await?;

				let runner_details = db::get_running_deployment_details_by_id(
					pool.acquire().await?.deref_mut(),
					&deployment.id,
				)
				.await?
				.status(500)?;

				if runner_details.runner_id == runner_id {
					Ok(Some(runner_details))
				} else {
					Err(Error::empty())
				}
			}
		}
	} else {
		Ok(None)
	}
}

async fn run_deployment_on_application_server(
	pool: &Pool<Database>,
	runner_id: &[u8],
	deployment: &mut Deployment,
	server: &IpAddr,
	settings: &Settings,
) -> Result<(), Error> {
	// now that there's a server available, mark the running of the server,
	// open a reverse tunnel, and run the image using a docker socket on it.

	// open reverse tunnel
	let socket =
		format!("./{}-{}-docker.sock", hex::encode(&deployment.id), server);
	if fs::metadata(&socket).await.is_ok() {
		let _ = fs::remove_file(&socket).await;
	}
	let _command = retry_with_delay_or_fail(
		|| async {
			let command = Command::new("ssh")
				.arg(format!("deployment@{}", server))
				.arg("-L")
				.arg(format!("{}:/var/run/docker.sock", socket))
				.kill_on_drop(true)
				.stdin(Stdio::piped())
				.stdout(Stdio::piped())
				.stderr(Stdio::piped())
				.spawn()?;

			Ok(command)
		},
		5,
		500,
		"Unable to open reverse tunnel",
	)
	.await?;

	let docker = Docker::unix(socket);
	let container_name = format!("deployment-{}", hex::encode(&deployment.id));
	let mut assigned_port = 0;
	let mut last_updated = get_current_time_millis();

	// Monitor deployment here
	loop {
		if *SHOULD_EXIT.read().await {
			break;
		}
		// Check to make sure some other runner hasn't taken over this
		// deployment
		let is_runner_managing_deployment = retry_with_delay_or_exit(
			|| is_deployment_managed_by_runner(pool, &deployment.id, runner_id),
			250,
			"Error getting running deployment details",
		)
		.await?;
		if !is_runner_managing_deployment {
			break;
		}

		// Poll deployment
		let container_info =
			get_container_details(&docker, &container_name).await?;

		let new_deployment = retry_with_delay_or_exit(
			|| async {
				let deployment = db::get_deployment_by_id(
					pool.acquire().await?.deref_mut(),
					&deployment.id,
				)
				.await?;
				Ok(deployment)
			},
			250,
			"Unable to get deployment information",
		)
		.await?;
		if let Some(new_deployment) = new_deployment {
			*deployment = new_deployment;
		} else {
			break;
		}

		if let Some(info) = container_info {
			// A docker container is running
			let deployed_image = if let Some(image) = &deployment.deployed_image
			{
				image.as_str()
			} else {
				// deployed_image is null. Stop the docker container and quit
				delete_container(&docker, &container_name).await?;
				break;
			};

			if deployed_image != info.config.image {
				// Deployed image is different from running image. Rerun
				delete_container(&docker, &container_name).await?;
				let new_port = run_container_in_server(
					&docker,
					server,
					pool,
					settings,
					&container_name,
					deployed_image,
					1.0,
					1024 * 1024 * 1024, // 1 GiB
					[80],
				)
				.await?[0];

				if new_port != assigned_port {
					// Update NGINX
					update_nginx_with_domain(
						&format!("{}.vicara.tech", hex::encode(&deployment.id)),
						server,
						new_port,
						settings,
					)
					.await?;
				}
				assigned_port = new_port;
			}
		} else {
			// Container is not running.
			let deployed_image = if let Some(image) = &deployment.deployed_image
			{
				// If the deployed image not null, deploy with that image
				image.as_str()
			} else {
				// If the deployed image is null, exit
				break;
			};

			let new_port = run_container_in_server(
				&docker,
				server,
				pool,
				settings,
				&container_name,
				deployed_image,
				1.0,
				1024 * 1024 * 1024, // 1 GiB
				[80],
			)
			.await?[0];

			if new_port != assigned_port {
				// Update NGINX
				update_nginx_with_domain(
					&format!("{}.vicara.tech", hex::encode(&deployment.id)),
					server,
					new_port,
					settings,
				)
				.await?;
			}
			assigned_port = new_port;
		}
		let (cpu, memory) = retry_with_delay_or_fail(
			|| async {
				// Get stats of container here
				let mut stats =
					docker.containers().get(&container_name).stats();
				let prestats = stats.next().await.status(500)??;
				let stats = stats.next().await.status(500)??;

				// Ref: https://docs.docker.com/engine/api/v1.41/#operation/ContainerStats
				let cpu_delta = (stats.cpu_stats.cpu_usage.total_usage -
					prestats.cpu_stats.cpu_usage.total_usage)
					as f64;
				let num_cpus = 1f64;
				let system_cpu_delta = (stats.cpu_stats.system_cpu_usage -
					prestats.cpu_stats.system_cpu_usage)
					as f64;
				let used_memory = (stats.memory_stats.usage -
					stats.memory_stats.stats.cache) as f64;
				let available_memory = stats.memory_stats.limit as f64;

				Ok((
					(cpu_delta / system_cpu_delta) * num_cpus * 100.0,
					(used_memory / available_memory) * 100.0,
				))
			},
			5,
			100,
			"Error polling deployment",
		)
		.await?;

		let _ = retry_with_delay_or_fail(
			|| async {
				db::add_deployment_running_stats(
					pool.acquire().await?.deref_mut(),
					&deployment.id,
					cpu,
					memory,
					get_current_time_millis(),
				)
				.await?;
				Ok(())
			},
			5,
			250,
			"Error updating running deployment stats",
		)
		.await;

		for _ in 0..=3 {
			let time_to_sleep = (
				last_updated + (2500)
				// 10 seconds
			)
			.checked_sub(get_current_time_millis());
			if let Some(time) = time_to_sleep {
				time::sleep(Duration::from_millis(time)).await;
			} else {
				// It took too long to poll deployment. Is the server being
				// overloaded?
				log::error!(
					"Time to sleep is negative. The server might be overloaded"
				);
			}
			last_updated = retry_with_delay_or_exit(
				|| async {
					let time = get_current_time_millis();
					db::update_running_deployment_last_updated(
						pool.acquire().await?.deref_mut(),
						time,
						&deployment.id,
						runner_id,
					)
					.await?;

					Ok(time)
				},
				250,
				"Error updating last updated for deployment",
			)
			.await?;
		}
	}
	if let Err(error) = db::delete_running_deployment_details(
		pool.acquire().await?.deref_mut(),
		&deployment.id,
	)
	.await
	{
		log::error!("Error deleting deployment running details: {}", error);
		time::sleep(Duration::from_millis(1000 * 10)).await;
	}

	Ok(())
}

async fn is_deployment_managed_by_runner(
	pool: &Pool<Database>,
	deployment_id: &[u8],
	runner_id: &[u8],
) -> Result<bool, Error> {
	let runner_details = db::get_running_deployment_details_by_id(
		pool.acquire().await?.deref_mut(),
		deployment_id,
	)
	.await?;
	if let Some(runner) = runner_details {
		Ok(runner.runner_id == runner_id)
	} else {
		Ok(false)
	}
}

async fn retry_with_delay_or_exit<TFunction, TFuture, TReturn, TMessage>(
	function: TFunction,
	delay_ms: u64,
	error_message: TMessage,
) -> Result<TReturn, Error>
where
	TFunction: Fn() -> TFuture,
	TFuture: Future<Output = Result<TReturn, Error>>,
	TMessage: AsRef<str>,
{
	loop {
		break match function().await {
			Ok(value) => Ok(value),
			Err(error) => {
				log::error!(
					"{}: {}",
					error_message.as_ref(),
					error.get_error()
				);
				if *SHOULD_EXIT.read().await {
					Err(Error::empty())
				} else {
					time::sleep(Duration::from_millis(delay_ms)).await;
					continue;
				}
			}
		};
	}
}

async fn retry_with_delay_or_fail<TFunction, TFuture, TReturn, TMessage>(
	function: TFunction,
	max_tries: u8,
	delay_ms: u64,
	error_message: TMessage,
) -> Result<TReturn, Error>
where
	TFunction: Fn() -> TFuture,
	TFuture: Future<Output = Result<TReturn, Error>>,
	TMessage: AsRef<str>,
{
	let mut error_count = 0;
	loop {
		break match function().await {
			Ok(value) => Ok(value),
			Err(error) => {
				log::error!(
					"{}: {}",
					error_message.as_ref(),
					error.get_error()
				);
				error_count += 1;
				if error_count >= max_tries {
					Err(error)
				} else {
					time::sleep(Duration::from_millis(delay_ms)).await;
					continue;
				}
			}
		};
	}
}

async fn get_available_port_on_server(server: &IpAddr) -> u32 {
	// Assign a random, available port
	let low = 1025;
	let high = 65535;
	let restricted_ports = [9000];
	let mut assigned_port;
	loop {
		assigned_port = rand::thread_rng().gen_range(low..high);
		if restricted_ports.contains(&assigned_port) {
			continue;
		}
		let port_open =
			is_port_open(format!("{}:{}", server, assigned_port)).await;
		if port_open {
			continue;
		}
		break;
	}
	assigned_port
}

async fn is_port_open<A: ToSocketAddrs>(addr: A) -> bool {
	TcpStream::connect(addr).await.is_ok()
}

async fn delete_container(
	docker: &Docker,
	container_name: &str,
) -> Result<(), Error> {
	retry_with_delay_or_exit(
		|| async {
			docker
				.containers()
				.get(container_name)
				.stop(Some(Duration::from_secs(30)))
				.await?;

			Ok(())
		},
		250,
		"Error stopping and deleting container",
	)
	.await
}

async fn get_container_details(
	docker: &Docker,
	container_name: &str,
) -> Result<Option<ContainerDetails>, Error> {
	retry_with_delay_or_fail(
		|| async {
			let inspect =
				docker.containers().get(container_name).inspect().await;
			if let Err(ShipliftError::Fault { code, .. }) = inspect {
				if code.as_u16() == 404 {
					// Container doesn't exist
					Ok(None)
				} else {
					Err(Error::empty())
				}
			} else if let Ok(details) = inspect {
				Ok(Some(details))
			} else {
				Err(Error::empty())
			}
		},
		5,
		250,
		"Error polling deployment",
	)
	.await
}

async fn run_container_in_server<const PORT_COUNT: usize>(
	docker: &Docker,
	server: &IpAddr,
	pool: &Pool<Database>,
	config: &Settings,
	container_name: &str,
	image: &str,
	cpu_limit: f64,
	memory_limit: u64,
	exposed_ports: [u32; PORT_COUNT],
) -> Result<[u32; PORT_COUNT], Error> {
	retry_with_delay_or_fail(
		|| async {
			let mut mapped_ports = [0; PORT_COUNT];

			let god_user = db::get_user_by_user_id(
				pool.acquire().await?.deref_mut(),
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
					name: if let Some((repo, _)) = image.split_once(':') {
						repo
					} else if let Some((repo, _)) = image.split_once('@') {
						repo
					} else {
						image
					}
					.replace("registry.vicara.tech/", ""),
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
					.image(image)
					.auth(registry_auth)
					.build(),
			);
			while stream.next().await.is_some() {}

			let mut builder = ContainerOptions::builder(image);
			builder
				.name(container_name)
				.auto_remove(true)
				.cpus(cpu_limit)
				.memory(memory_limit)
				.privileged(false);
			for (index, port) in exposed_ports.iter().enumerate() {
				let mapped_port = get_available_port_on_server(server).await;
				mapped_ports[index] = mapped_port;
				builder.expose(*port, "tcp", mapped_port);
			}

			docker.containers().create(&builder.build()).await?;
			docker.containers().get(container_name).start().await?;

			Ok(mapped_ports)
		},
		5,
		1000,
		"Error creating container",
	)
	.await
}

async fn update_nginx_with_domain(
	domain: &str,
	server: &IpAddr,
	port: u32,
	settings: &Settings,
) -> Result<(), Error> {
	retry_with_delay_or_fail(
		|| update_dns(domain, settings),
		5,
		1000,
		"Unable to update DNS",
	)
	.await?;

	retry_with_delay_or_fail(
		|| update_nginx(domain, server, port),
		5,
		1000,
		"Unable to update DNS",
	)
	.await?;

	Ok(())
}

async fn update_dns(domain: &str, settings: &Settings) -> Result<(), Error> {
	let credentials = Credentials::UserAuthToken {
		token: settings.cloudflare.api_token.clone(),
	};
	let client = if let Ok(client) = Client::new(
		credentials,
		HttpApiClientConfig::default(),
		Environment::Production,
	) {
		client
	} else {
		return Err(Error::empty());
	};
	let response = client
		.request(&ListZones {
			params: ListZonesParams {
				name: Some(String::from("vicara.tech")),
				..Default::default()
			},
		})
		.await?;
	let zone = response.result.into_iter().next().status(500)?;
	let zone_identifier = zone.id.as_str();
	let expected_dns_record = DnsContent::CNAME {
		content: String::from("proxy.vicara.tech"),
	};

	let response = client
		.request(&ListDnsRecords {
			zone_identifier,
			params: ListDnsRecordsParams {
				name: Some(String::from(domain)),
				..Default::default()
			},
		})
		.await?;
	let dns_record = response.result.into_iter().find(|record| {
		if let DnsContent::CNAME { .. } = record.content {
			record.name == domain
		} else {
			false
		}
	});
	if let Some(record) = dns_record {
		if let DnsContent::CNAME { content } = record.content {
			if content != "proxy.vicara.tech" {
				client
					.request(&UpdateDnsRecord {
						zone_identifier,
						identifier: record.id.as_str(),
						params: UpdateDnsRecordParams {
							content: expected_dns_record,
							name: domain,
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
					name: domain.replace("vicara.tech", "").as_str(),
					ttl: Some(1),
					priority: None,
					proxied: Some(true),
				},
			})
			.await?;
	}

	Ok(())
}

async fn update_nginx(
	domain: &str,
	server: &IpAddr,
	port: u32,
) -> Result<(), Error> {
	let session = retry_with_delay_or_fail(
		|| async {
			let session = Session::connect(
				"ssh://root@proxy.vicara.tech",
				KnownHosts::Add,
			)
			.await?;
			Ok(session)
		},
		5,
		1000,
		"Unable to connect to proxy server",
	)
	.await?;

	let mut sftp = session.sftp();
	let result = sftp
		.read_from(format!("/etc/letsencrypt/live/{}/fullchain.pem", domain))
		.await;
	if let Err(openssh::Error::Remote(error)) = result {
		if error.kind() == ErrorKind::NotFound {
			// File doesn't exist
			// Generate certs
			let mut writer = sftp
				.write_to(format!("/etc/nginx/sites-enabled/{}", domain))
				.await?;
			writer
				.write_all(
					format!(
						r#"
server {{
	listen 80;
	listen [::]:80;
	server_name {domain};

	location {path} {{
		proxy_pass http://{server}:{port};
	}}

	include snippets/letsencrypt.conf;
}}
"#,
						domain = domain,
						port = port,
						path = "/",
						server = server,
					)
					.as_bytes(),
				)
				.await?;

			let reload_result = session
				.command("nginx")
				.arg("-s")
				.arg("reload")
				.spawn()?
				.wait()
				.await?;
			if !reload_result.success() {
				return Err(Error::empty());
			}

			let certificate_result = session
				.command("certbot")
				.arg("certonly")
				.arg("--agree-tos")
				.arg("-m")
				.arg("postmaster@vicara.co")
				.arg("--no-eff-email")
				.arg("-d")
				.arg(&domain)
				.arg("--webroot")
				.arg("-w")
				.arg("/var/www/letsencrypt")
				.spawn()?
				.wait()
				.await?;
			if !certificate_result.success() {
				return Err(Error::empty());
			}
		}
		return Err(error.into());
	}

	// Certs exist. Continue
	let mut writer = sftp
		.write_to(format!("/etc/nginx/sites-enabled/{}", domain))
		.await?;
	writer
		.write_all(
			format!(
				r#"
server {{
	listen 80;
	listen [::]:80;
	server_name {domain};

	return 301 https://{domain}$request_uri$is_args$args;
}}

server {{
	listen 443 ssl http2;
	listen [::]:443 ssl http2;
	server_name {domain};

	ssl_certificate /etc/letsencrypt/live/{domain}/fullchain.pem;
	ssl_certificate_key /etc/letsencrypt/live/{domain}/privkey.pem;

	location {path} {{
		proxy_set_header   X-Forwarded-Proto $scheme;
		proxy_set_header   Host              $host;
		proxy_set_header   X-Real-IP         $remote_addr;
		proxy_set_header   X-Forwarded-For   $proxy_add_x_forwarded_for;
		proxy_pass http://{server}:{port};
	}}

	include snippets/letsencrypt.conf;
}}
"#,
				domain = domain,
				port = port,
				path = "/",
				server = server,
			)
			.as_bytes(),
		)
		.await?;
	writer.close().await?;
	drop(sftp);

	let reload_result = session
		.command("nginx")
		.arg("-s")
		.arg("reload")
		.spawn()?
		.wait()
		.await?;
	if !reload_result.success() {
		return Err(Error::empty());
	}

	Ok(())
}
