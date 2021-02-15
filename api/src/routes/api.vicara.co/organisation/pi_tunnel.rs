use crate::{
	app::{create_eve_app, App},
	db, error,
	models::rbac::{self, permissions},
	pin_fn,
	utils::{
		constants::request_keys, get_current_time, EveContext, EveMiddleware,
	},
};
use eve_rs::{App as EveApp, Context, Error, NextHandler};
use futures::StreamExt;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde_json::{json, Value};
use shiplift::builder::ExecContainerOptions;
use shiplift::{ContainerOptions, Docker};
use std::{
	env, io,
	path::{Path, PathBuf},
	str::from_utf8,
};
use tokio::{fs, io::AsyncWriteExt};

/// END POINTS TO BE ADDED.
/// addUser/
/// getBashScript/
/// deleteTunnel/

// todo: implement token auth.
pub fn creare_sub_app(app: &App) -> EveApp<EveContext, EveMiddleware, App> {
	let mut sub_app = create_eve_app(app);

	sub_app.post(
		"/:organisationId/create",
		&[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::pi_tunnel::ADD,
				api_macros::closure_as_pinned_box!(|mut context| {
					let org_id_string =
						context.get_param(request_keys::ORGANISATION_ID);

					if org_id_string.is_none() {
						context.status(400).json(error!(WRONG_PARAMETERS));
						return Ok((context, None));
					}

					let org_id_string = org_id_string.unwrap();
					let organisation_id = hex::decode(&org_id_string);
					if organisation_id.is_err() {
						context.status(400).json(error!(WRONG_PARAMETERS));
						return Ok((context, None));
					}
					let organisation_id = organisation_id.unwrap();

					let resource = db::get_resource_by_id(
						context.get_mysql_connection(),
						&organisation_id,
					)
					.await?;
					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(create)),
		],
	);

	sub_app.post(
		"/:organisationId/get-bash-script",
		&[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::pi_tunnel::VIEW,
				api_macros::closure_as_pinned_box!(|mut context| {
					let org_id_string =
						context.get_param(request_keys::ORGANISATION_ID);

					if org_id_string.is_none() {
						log::debug!("org id is none");
						context.status(400).json(error!(WRONG_PARAMETERS));
						return Ok((context, None));
					}

					let org_id_string = org_id_string.unwrap();

					let organisation_id = hex::decode(&org_id_string);
					if organisation_id.is_err() {
						log::debug!("could not decoide org id");
						context.status(400).json(error!(WRONG_PARAMETERS));
						return Ok((context, None));
					}
					let organisation_id = organisation_id.unwrap();

					let resource = db::get_resource_by_id(
						context.get_mysql_connection(),
						&organisation_id,
					)
					.await?;

					if resource.is_none() {
						log::debug!("resource is none");
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(get_bash_script)),
		],
	);
	sub_app
}

async fn create(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let body = context.get_body_object().clone();
	// get tunnel name
	let tunnel_name = if let Some(Value::String(tunnel_name)) =
		body.get(request_keys::TUNNEL_NAME)
	{
		tunnel_name
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	// get user name and user id from tokendata
	let username = &context.get_token_data().unwrap().user.username.clone();
	let user_id = context.get_param(request_keys::ORGANISATION_ID).unwrap();
	let user_id = hex::decode(&user_id).unwrap();
	log::debug!("Generated user is is {:#?}", user_id);

	// generate new resource id for the generated container.
	let id = db::generate_new_resource_id(context.get_mysql_connection()).await;
	if let Err(resource_id_error) = id {
		log::error!("Error occured while generating resource id for pi tunnel");
		context.status(500).json(error!(SERVER_ERROR));
		return Ok(context);
	}
	let id = id.unwrap();
	let id = id.as_bytes(); // convert to byte array
	log::debug!("Generated resource id is {:#?}", &id);

	// generate unique password
	let generated_password = generate_password(10);

	// create container
	let docker = Docker::new();
	let image = get_docker_image_name();
	let image = image.as_str();
	let container_name = get_container_name(username.as_str());
	let volume_path = get_volume_path(username.as_str());
	let volumes = vec![&volume_path[..]];
	let host_ssh_port = get_port();
	let exposed_port = get_exposed_port();
	let server_ssh_port = get_ssh_port_for_server();
	let host_listen_port = get_port();
	let server_ip_address = get_server_ip_address();

	// check if container name already exists
	let is_container_available = db::check_if_container_exists(
		context.get_mysql_connection(),
		&container_name,
	)
	.await;
	if let Err(_container_check_err) = is_container_available {
		context.status(500).json(error!(SERVER_ERROR));
		return Ok(context);
	};
	let is_container_available = is_container_available.unwrap();
	if is_container_available.is_some() {
		log::error!("Container with the name already exists");
		context.status(500).json(error!(SERVER_ERROR));
		return Ok(context);
	}

	match docker
		.containers()
		.create(
			&ContainerOptions::builder(image.as_ref())
				.name(&container_name)
				.volumes(volumes)
				.env(vec![
					format!("name={}", &container_name).as_str(),
					format!("SUDO_ACCESS={}", true).as_str(),
					format!("PASSWORD_ACCESS={}", true).as_str(),
					format!("USER_PASSWORD={}", &generated_password).as_str(),
					format!("USER_NAME={}", &username).as_str(),
				])
				.expose(server_ssh_port, "tcp", host_ssh_port)
				.expose(exposed_port, "tcp", host_listen_port)
				.build(),
		)
		.await
	{
		Err(docker_error) => {
			log::error!(
				"Could not create docker image. error => {:#?}",
				docker_error
			);
			context.status(500).json(error!(SERVER_ERROR));
			return Ok(context);
		}
		Ok(container_info) => {
			log::debug!("fectching docker information...");
			let container_id = container_info.id;
			if let Err(container_start_error) =
				docker.containers().get(&container_id).start().await
			{
				// one possibility of failing is when the contaier with the given name already exists`
				// so maybe, delete the container here?
				log::error!("{:?}", container_start_error);
				context.status(500).json(error!(SERVER_ERROR));
				return Ok(context);
			}

			// store data in db

			// create resource in db
			if let Err(resource_err) = db::create_resource(
				context.get_mysql_connection(),
				id,
				&format!("PiTunnel-container-{}", tunnel_name),
				rbac::RESOURCE_TYPES
					.get()
					.unwrap()
					.get(rbac::resource_types::PI_TUNNEL)
					.unwrap(),
				&user_id,
			)
			.await
			{
				log::error!("error for org query{:?}", resource_err);
			}

			//  add container information in pi_tunnel table
			if let Err(database_error) = db::add_user_for_pi_tunnel(
				context.get_mysql_connection(),
				&id[..],
				username.as_str(),
				host_ssh_port,
				exposed_port,
				&container_name.as_str(),
			)
			.await
			{
				log::error!(
					"Error while adding data to pi_tunnel table. {:#?}",
					database_error
				);

				// if there is an error in database query, stop the container which just started.
				log::info!("Stopping container {} ...", &container_name);
				if let Err(container_stop_error) =
					docker.containers().get(&container_id).stop(None).await
				{
					log::error!(
						"Error while stopping the container. Error => {:?}",
						container_stop_error
					);
				} else {
					log::info!("Deleting container...");
					// delete the container
					if let Err(container_delete_error) =
						docker.containers().get(&container_id).delete().await
					{
						log::error!(
							"could not stop delete container. Error => {:#?}",
							container_delete_error
						);
					}
				}
				context.status(500).json(error!(SERVER_ERROR));
				return Ok(context);
			};
		}
	}

	context.json(json!({
		request_keys::SUCCESS : true,
		request_keys::USERNAME : &username,
		request_keys::PASSWORD : &generated_password,
		request_keys::SERVER_IP_ADDRESS : &server_ip_address,
		request_keys::SSH_PORT : host_ssh_port,
		request_keys::HOST_LISTENING_PORT : host_listen_port,
		request_keys::EXPOSED_PORT : vec![exposed_port],
	}));
	// on success, return ssh port, username,  exposed port, server ip address, password
	Ok(context)
}

// {
// 	localPort,
// 	localHostName,
// 	exposedServerPort,
//	serverSSHPort
// 	serverIPAddress,
// 	serverUserName
// }
async fn get_bash_script(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	log::debug!("inside bash script func");
	// get request body
	let body = context.get_body_object().clone();
	let local_port = if let Some(Value::String(local_port)) =
		body.get(request_keys::LOCAL_PORT)
	{
		local_port
	} else {
		log::debug!("failed at local port");
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let local_host_name = if let Some(Value::String(local_host_name)) =
		body.get(request_keys::LOCAL_HOST_NAME)
	{
		local_host_name
	} else {
		log::debug!("failed at local host name");
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let exposed_server_port = if let Some(Value::String(exposed_server_port)) =
		body.get(request_keys::EXPOSED_SERVER_PORT)
	{
		exposed_server_port
	} else {
		log::debug!("here is exposed server port");
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let server_ip_address = if let Some(Value::String(server_ip_address)) =
		body.get(request_keys::SERVER_IP_ADDRESS)
	{
		server_ip_address
	} else {
		log::debug!("failed at server ip address");
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let server_user_name = if let Some(Value::String(server_user_name)) =
		body.get(request_keys::SERVER_USER_NAME)
	{
		server_user_name
	} else {
		log::debug!("failed at server user name");
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let server_ssh_port = if let Some(Value::String(server_ssh_port)) =
		body.get(request_keys::SERVER_SSH_PORT)
	{
		server_ssh_port
	} else {
		log::debug!("here is exposed server port");
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let bash_script_file_content = bash_script_formatter(
		local_port,
		local_host_name,
		exposed_server_port,
		server_ip_address,
		server_user_name,
		server_ssh_port,
	)
	.await;
	if let Err(_error) = bash_script_file_content {
		context.status(500).json(error!(SERVER_ERROR));
		return Ok(context);
	}
	let bash_script_file_content = bash_script_file_content.unwrap();

	// set attachment type
	// context.attachment(Some(&"connect-pi-to-server.sh".to_owned()[..]));
	// context.body(&bash_script_file_content[..]);
	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::SCRIPT : bash_script_file_content,
	}));

	Ok(context)
}

// UTIL FUNCTIONS

///generates random password of given length.
fn generate_password(length: u16) -> String {
	let password: String = thread_rng()
		.sample_iter(&Alphanumeric)
		.take(length.into())
		.map(char::from)
		.collect();

	return password;
}

///reads the script file, replaces given data and returns file content as String.
async fn bash_script_formatter(
	local_port: &String,
	local_host_name: &String,
	exposed_server_port: &String,
	server_ip_address: &String,
	server_user_name: &String,
	server_ssh_port: &String,
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
fn get_bash_script_path() -> std::io::Result<PathBuf> {
	Ok(env::current_dir()?
		.join("assets")
		.join("pi_tunnel")
		.join("connect-pi-to-server.sh"))
}

/// reads user data file and replaces username and password with the given values
async fn get_updated_user_data(
	username: &str,
	password: &str,
) -> std::io::Result<String> {
	let path = get_user_data_file_path()?;
	let mut contents = fs::read_to_string(path).await?;
	contents = contents
		.replace("usernameVariable", username)
		.replace("passwordVariable", password);
	Ok(contents)
}

/// returns path to user-data file
fn get_user_data_file_path() -> std::io::Result<PathBuf> {
	Ok(env::current_dir()?
		.join("assets")
		.join("pi_tunnel")
		.join("user-data"))
}

/// cretes user data file at /home/web/pi_tunnel
async fn create_user_data_file(
	username: &str,
	file_content: String,
) -> io::Result<()> {
	let mut filename = String::from(username);
	// generate file name
	filename.push_str("-user-data");
	//generate path
	let path = Path::new("/home")
		.join("web")
		.join("pi-tunnel")
		.join(filename);

	// create and write to the file
	fs::File::create(path)
		.await?
		.write_all(file_content.as_bytes())
		.await
}

fn get_exposed_port() -> u32 {
	return 8081;
}

fn get_port() -> u32 {
	return generate_port();
}

fn get_ssh_port_for_server() -> u32 {
	return 2222;
}

fn get_container_name(username: &str) -> String {
	let mut container_name = String::from(username);
	container_name.push_str("-container");
	return container_name;
}

fn get_docker_image_name() -> String {
	let image = "new-image:1.0";
	return String::from(image);
}

fn get_volume_path(username: &str) -> String {
	let mut volume_path = String::from("/home/web/pi-tunnel/");
	volume_path.push_str(username);
	volume_path.push_str("-user-data");
	volume_path.push_str(":/temp/user-data");
	return volume_path;
}

// generates valid port
pub fn generate_port() -> u32 {
	let low = 1025;
	let high = 65535;
	let port = rand::thread_rng().gen_range(low, high);

	if !is_valid_port(port, low, high) {
		return generate_port();
	}

	return port as u32;
}

/// returns true if port is valid
pub fn is_valid_port(generated_port: i32, low: i32, high: i32) -> bool {
	let restricted_ports = vec![5800, 8080, 9000];
	if restricted_ports.iter().any(|&port| port == generated_port) {
		return false;
	}
	// check port between 1-1024 && 5900 - 5910s
	if generated_port <= low
		|| (generated_port >= 5900 && generated_port <= 5910)
	{
		return false;
	}
	return true; // valid port
}

pub fn get_server_ip_address() -> String {
	return String::from("143.110.179.80");
}
// queries for pi tunnel table

// CREATE TABLE IF NOT EXISTS pi_tunnel (id binary(16),username varchar(100), sshPort integer, exposedPort integer, containerId varchar(50));
// ALTER TABLE pi_tunnel ADD PRIMARY KEY (`id`);
