use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac,
	pin_fn,
	utils::{constants::request_keys, EveContext, EveMiddleware},
};
use eve_rs::{App as EveApp, Context, Error, NextHandler};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde_json::{json, Value};
use std::{env, io, path::PathBuf};
use tokio::fs;

/// END POINTS TO BE ADDED.
/// addUser/
/// getBashScript/
/// deleteTunnel/

// todo: implement token auth.
pub fn creare_sub_app(app: &App) {
	let mut sub_app = create_eve_app(app);

	sub_app.post(
		"/add-user",
		&[EveMiddleware::CustomFunction(pin_fn!(add_user))],
	);

	sub_app.get(
		"/get-bash-script",
		&[EveMiddleware::CustomFunction(pin_fn!(get_bash_script))],
	);
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
	let username = &context.get_token_data().unwrap().user;

	// generate unique password
	let generated_password = generate_password(10);

	// get docker image and expose port
	// call script to add usern in the image
	// upon success, return credentials

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

/// generates random password of given length.
fn generate_password(length: u16) -> String {
	let password: String = thread_rng()
		.sample_iter(&Alphanumeric)
		.take(length.into())
		.map(char::from)
		.collect();

	return password;
}

/// reads the script file, replaces given data and returns file content as String.
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
		.replace("localPortVariable", local_host_name)
		.replace("localHostNameVaribale", local_host_name)
		.replace("serverPortVariable", server_port)
		.replace("serverHostNameOrIpAddressVariable", server_ip_address)
		.replace("serverUserNameVariable", server_user_name);

	Ok(contents)
}

/// returns path to the script file.
fn get_bash_script_path() -> std::io::Result<PathBuf> {
	Ok(env::current_dir()?
		.join("src")
		.join("assets")
		.join("pi_tunnel")
		.join("connection-to-pi-server.sh"))
}
