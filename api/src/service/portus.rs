// SERVICE FOR PORTUS
use crate::db;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use shiplift::{Docker, Error};
use sqlx::{MySql, Transaction};
use std::{env, path::PathBuf};
use tokio::fs;

// function to stop a container
pub async fn delete_container(
	docker: Docker,
	container_name: &str,
) -> Result<(), Error> {
	let container_stop_result =
		docker.containers().get(&container_name).stop(None).await;

	let container_delete_result =
		docker.containers().get(&container_name).delete().await;

	if container_delete_result.is_err() {
		let container_start_result =
			docker.containers().get(&container_name).start().await;
		if let Err(err) = container_start_result {
			log::error!(
				"Unrecoverable error while starting container: {:?}",
				err
			);

			return Err(err);
		}
	}
	if let Err(err) = container_stop_result.and(container_delete_result) {
		log::error!("Error while the container {:?}", err);
		return Err(err);
	}

	Ok(())
}

// function to assign available port
pub fn generate_password(length: u16) -> String {
	thread_rng()
		.sample_iter(&Alphanumeric)
		.take(length.into())
		.collect()
}

// generates username for portus user
pub fn generate_username(length: u16) -> String {
	const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz";

	(0..length)
		.map(|_| {
			let idx = thread_rng().gen_range(0, CHARSET.len());
			CHARSET[idx] as char
		})
		.collect()
}

// formats bash script with given parameters
pub async fn bash_script_formatter(
	local_port: &str,
	local_host_name: &str,
	exposed_server_port: &str,
	server_ip_address: &str,
	server_user_name: &str,
	server_ssh_port: &str,
) -> std::io::Result<String> {
	// get script file path
	let path = get_bash_script_path()?;
	let mut contents = fs::read_to_string(path).await?;
	contents = contents
		.replace("localPortVariable", local_port)
		.replace("localHostNameVaribale", local_host_name)
		.replace("exposedServerPortVariable", exposed_server_port)
		.replace("serverHostNameOrIpAddressVariable", server_ip_address)
		.replace("serverUserNameVariable", server_user_name)
		.replace("serverSSHPortVariable", server_ssh_port);

	Ok(contents)
}

/// returns path to the script file.
pub fn get_bash_script_path() -> std::io::Result<PathBuf> {
	Ok(env::current_dir()?
		.join("assets")
		.join("portus")
		.join("connect-pi-to-server.sh"))
}

pub fn get_ssh_port_for_server() -> u32 {
	return 2222;
}

pub fn get_container_name(username: &str) -> String {
	format!("{}-container", username)
}

pub fn get_server_ip_address() -> &'static str {
	"143.110.179.80"
}

// generates valid port
pub async fn assign_available_port(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<u32, sqlx::Error> {
	let low = 1025;
	let high = 65535;
	let restricted_ports = [5800, 8080, 9000];

	loop {
		let port = rand::thread_rng().gen_range(low, high);
		if restricted_ports.contains(&port) {
			continue;
		}
		if port >= 5900 && port <= 5910 {
			continue;
		}
		let port_available =
			db::is_portus_port_available(transaction, port).await?;
		if !port_available {
			continue;
		}
		return Ok(port);
	}
}
