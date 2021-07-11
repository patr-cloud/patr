// SERVICE FOR PORTUS
use std::{env, iter, path::PathBuf};

use rand::{distributions::Alphanumeric, prelude::*};
use tokio::fs;

use crate::{db, Database};

/// # Description
/// This function is used to generate password for portus user
///
/// # Arguments
/// * `length` - an unsigned size variable used to define size of new password
///
/// # Returns
/// This function returns a string containing newly generated password
pub fn generate_password(length: usize) -> String {
	let mut rng = thread_rng();
	iter::repeat(())
		.map(|()| rng.sample(Alphanumeric))
		.map(char::from)
		.take(length)
		.collect()
}

/// # Description
/// This function is used to generate username for portus user
///
/// # Arguments
/// * `length` - an unsigned size variable containing the size of username of
/// user
///
/// # Returns
/// This function returns a string containing newly generated username
pub fn generate_username(length: u16) -> String {
	const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz";

	(0..length)
		.map(|_| {
			let idx = thread_rng().gen_range(0..CHARSET.len());
			CHARSET[idx] as char
		})
		.collect()
}

/// # Description
/// the function that is used to format bash script with given parameters
///
/// # Arguments
/// * `local_port` - string containing local post variable name
/// * `local_host_name` - string containing local hostname variable
/// * `exposed_server_port` - string containing exposed server post variable
/// * `server_ip_address` - string containing server hostname or ip address
///   variable
/// * `server_user_name` - string containing server username variable
/// * `server_ssh_port` - string containing server SSH port variable
///
/// # Returns
/// This function returns Result<String> containing contents of bash command
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
		.replace("localHostNameVariable", local_host_name)
		.replace("exposedServerPortVariable", exposed_server_port)
		.replace("serverHostNameOrIpAddressVariable", server_ip_address)
		.replace("serverUserNameVariable", server_user_name)
		.replace("serverSSHPortVariable", server_ssh_port);

	Ok(contents)
}

/// # Description
/// This function is used to get path to the script file.
///
/// # Returns
/// This function returns Result<PathBuf> containing path of script file or
/// nothing
pub fn get_bash_script_path() -> std::io::Result<PathBuf> {
	Ok(env::current_dir()?
		.join("assets")
		.join("portus")
		.join("connect-pi-to-server.sh"))
}

/// # Description
/// This function is used to get port for server
///
/// # Returns
/// This function returns an unsigned integer containing the ssh port for server
pub const fn get_ssh_port_for_server() -> u16 {
	2222
}

/// # Description
/// This function is used to generate the name of container
///
/// # Arguments
/// * `username` - a string containing username of user
///
/// # Returns
/// This function returns a string containing formatted container name
pub fn get_container_name(username: &str) -> String {
	format!("{}-container", username)
}

/// # Description
/// This function gets server ip address
///
/// # Returns
/// This function returns server ip address
pub const fn get_server_ip_address() -> &'static str {
	"143.110.179.80"
}

/// # Description
/// This function is used get an available port.
///
/// # Arguments
/// * `transaction` - database save point, more details here: [`Transaction`]
///
/// # Returns
/// This function returns `Result<u16, sqlx::Error>` containing unsigned 16 bit
/// integer containing the port number
/// [`Transaction`]: Transaction
pub async fn assign_available_port(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<u16, sqlx::Error> {
	let low = 1025;
	let high = 65535;
	let restricted_ports = [5800, 8080, 9000, 3000];

	loop {
		let port = rand::thread_rng().gen_range(low..high);
		if restricted_ports.contains(&port) {
			continue;
		}
		if (5900..=5910).contains(&port) {
			continue;
		}
		let port_available =
			db::is_portus_port_available(connection, port).await?;
		if !port_available {
			continue;
		}
		return Ok(port);
	}
}
