use crate::{
	app::{create_eve_app, App},
	db, error,
	models::rbac::{self, permissions},
	pin_fn,
	utils::{constants::request_keys, EveContext, EveMiddleware},
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
		"/:organisationId/add-user",
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
			EveMiddleware::CustomFunction(pin_fn!(add_user)),
		],
	);

	sub_app.post(
		"/:organisationId/get-bash-script",
		&[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::domain::LIST,
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
			EveMiddleware::CustomFunction(pin_fn!(get_bash_script)),
		],
	);
	sub_app
}

/// function to create new user in linux machine

// gets username from the context
// generates a random 10 character alphanumeric password for the given username
// adds the user to docker image
// sends back username, password, ipaddress, port(server) to login into, and port exposed in docker image.

async fn add_user(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	// get user name from tokendata
	let username = &context.get_token_data().unwrap().user.username.clone();
	let id = &context.get_token_data().unwrap().user.id.clone();

	// generate unique password
	let generated_password = generate_password(10);

	// get user data file
	let user_data_file_content =
		get_updated_user_data(username.as_str(), &generated_password).await;
	if let Err(user_data_error) = user_data_file_content {
		context.status(500).json(error!(SERVER_ERROR));
		return Ok(context);
	}
	let user_data_file_content = user_data_file_content.unwrap();

	// once updated content is received, create a file in home/web/pi_tunnel unique for the given user.
	// format for filename => <username>_user_data
	if let Err(create_file_error) =
		create_user_data_file(username.as_str(), user_data_file_content).await
	{
		context.status(500).json(error!(SERVER_ERROR));
		return Ok(context);
	}

	// create container
	let docker = Docker::new();
	let image = get_docker_image_name();
	let image = image.as_str();
	let container_name = get_container_name(username.as_str());
	let volume_path = get_volume_path(username.as_str());
	let volumes = vec![&volume_path[..]];
	let host_ssh_port = get_host_ssh_port();
	let exposed_port = get_exposed_port();
	let server_ssh_port = get_ssh_port_for_server();

	// check if container name already exists
	let is_container_available = db::check_if_container_exists(
		context.get_mysql_connection(),
		&container_name,
	)
	.await;
	if let Err(container_check_err) = is_container_available {
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
				.expose(2222, "tcp", host_ssh_port)
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
				log::error!("{:?}", container_start_error);
				context.status(500).json(error!(SERVER_ERROR));
				return Ok(context);
			}

			// once container is started, use exec to run bash command.
			let container_options = ExecContainerOptions::builder()
				.cmd(vec![
					"/bin/bash",
					"-c",
					"temp/create-user.sh && rm temp/create-user.sh",
				])
				.build();

			while let Some(docker_container) = docker
				.containers()
				.get(&container_id)
				.exec(&container_options)
				.next()
				.await
			{
				if let Err(docker_container_err) = docker_container {
					log::error!("error occured while executing exec for docker container. Err => {:?}", docker_container_err);
					context.status(500).json(error!(SERVER_ERROR));
					return Ok(context);
				}
			}

			// store data in db
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
				// if ther is error in database query, stop the container which just started.
				if let Err(docker_stop_error) =
					docker.containers().get(&container_id).stop(None).await
				{
					log::error!(
						"could not stop docker container. Error => {:#?}",
						docker_stop_error
					);
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
		request_keys::SERVER_IP_ADDRESS : "",
		request_keys::SSH_PORT : host_ssh_port,
		request_keys::EXPOSED_PORT : vec![exposed_port],
	}));
	// on success, return ssh port, username,  exposed port, server ip address, password
	Ok(context)
}

// {
// 	localPort,
// 	localHostName,
// 	serverPort,
// 	serverIPAddress,
// 	serverUserName
// }
async fn get_bash_script(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	// get request body
	let body = context.get_body_object().clone();
	let local_port = if let Some(Value::String(local_port)) =
		body.get(request_keys::LOCAL_PORT)
	{
		local_port
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let local_host_name = if let Some(Value::String(local_host_name)) =
		body.get(request_keys::LOCAL_HOST_NAME)
	{
		local_host_name
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let server_port = if let Some(Value::String(server_port)) =
		body.get(request_keys::SERVER_PORT)
	{
		server_port
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let server_ip_address = if let Some(Value::String(server_ip_address)) =
		body.get(request_keys::SERVER_IP_ADDRESS)
	{
		server_ip_address
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let server_user_name = if let Some(Value::String(server_user_name)) =
		body.get(request_keys::SERVER_USER_NAME)
	{
		server_user_name
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let bash_script_file_content = bash_script_formatter(
		local_port,
		local_host_name,
		server_port,
		server_ip_address,
		server_user_name,
	)
	.await;
	if let Err(error) = bash_script_file_content {
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
	server_port: &String,
	server_ip_address: &String,
	server_user_name: &String,
) -> std::io::Result<String> {
	// get script file path
	let path = get_bash_script_path()?;
	let mut contents = fs::read_to_string(path).await?;
	contents = contents
		.replace("localPortVariable", local_port)
		.replace("localHostNameVaribale", local_host_name)
		.replace("serverPortVariable", server_port)
		.replace("serverHostNameOrIpAddressVariable", server_ip_address)
		.replace("serverUserNameVariable", server_user_name);

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

fn get_host_ssh_port() -> u32 {
	return 8082;
}

fn get_ssh_port_for_server() -> u32 {
	return 4343;
}

fn get_container_name(username: &str) -> String {
	let mut container_name = String::from(username);
	container_name.push_str("-container");
	return container_name;
}

fn get_docker_image_name() -> String {
	let image = "third-tunnel:1.0";
	return String::from(image);
}

fn get_volume_path(username: &str) -> String {
	let mut volume_path = String::from("/home/web/pi-tunnel/");
	volume_path.push_str(username);
	volume_path.push_str("-user-data");
	volume_path.push_str(":/temp/user-data");
	return volume_path;
}
// queries for pi tunnel table

// CREATE TABLE IF NOT EXISTS pi_tunnel (id binary(16),username varchar(100), sshPort integer, exposedPort integer, containerId varchar(50));
// ALTER TABLE pi_tunnel ADD PRIMARY KEY (`id`);
