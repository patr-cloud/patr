// SERVICE FOR PORTUS
use std::{env, iter, path::PathBuf};

use rand::{distributions::Alphanumeric, prelude::*};
use sqlx::{MySql, Transaction};
use tokio::fs;

use crate::db;

/// function to assign available port
pub fn generate_password(length: usize) -> String {
	let mut rng = thread_rng();
	iter::repeat(())
		.map(|()| rng.sample(Alphanumeric))
		.map(char::from)
		.take(length)
		.collect()
}

/// generates username for portus user
pub fn generate_username(length: u16) -> String {
	const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz";

	(0..length)
		.map(|_| {
			let idx = thread_rng().gen_range(0..CHARSET.len());
			CHARSET[idx] as char
		})
		.collect()
}

/// formats bash script with given parameters
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

/// function to get an available port.
pub async fn assign_available_port(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<u32, sqlx::Error> {
	let low = 1025;
	let high = 65535;
	let restricted_ports = [5800, 8080, 9000, 3000];

	loop {
		let port = rand::thread_rng().gen_range(low..high);
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
