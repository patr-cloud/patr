// SERVICE FOR PORTUS
use std::{env, path::PathBuf};

use rand::{distributions::Alphanumeric, thread_rng, Rng};
use shiplift::{Docker, Error};
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

pub fn generate_username(length: u16) -> String {
	const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz";

	(0..length)
		.map(|_| {
			let idx = thread_rng().gen_range(0, CHARSET.len());
			CHARSET[idx] as char
		})
		.collect()
}

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
