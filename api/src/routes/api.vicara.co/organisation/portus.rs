use std::error::Error as StdError;

use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use serde_json::{json, Value};
use shiplift::{ContainerOptions, Docker};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac::{self, permissions},
	pin_fn,
	service,
	utils::{
		constants::{self, request_keys},
		get_current_time,
		ErrorData,
		EveContext,
		EveError as Error,
		EveMiddleware,
	},
};

pub fn creare_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut sub_app = create_eve_app(app);

	// list tunnels under an org
	sub_app.get(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::portus::LIST,
				api_macros::closure_as_pinned_box!(|mut context| {
					let org_id =
						context.get_param(request_keys::ORGANISATION_ID);

					if org_id.is_none() {
						context.status(400).json(error!(WRONG_PARAMETERS));
						return Ok((context, None));
					}

					let org_id = org_id.unwrap();
					let organisation_id = hex::decode(&org_id);
					if organisation_id.is_err() {
						context.status(400).json(error!(WRONG_PARAMETERS));
						return Ok((context, None));
					}
					let organisation_id = organisation_id.unwrap();

					let resource = db::get_resource_by_id(
						context.get_mysql_connection(),
						&organisation_id,
					)
					.await
					.status(500)
					.body(error!(SERVER_ERROR).to_string())?;
					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(
				get_tunnels_for_organisation
			)),
		],
	);

	sub_app.post(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::portus::ADD,
				api_macros::closure_as_pinned_box!(|mut context| {
					let org_id =
						context.get_param(request_keys::ORGANISATION_ID);

					if org_id.is_none() {
						context.status(400).json(error!(WRONG_PARAMETERS));
						return Ok((context, None));
					}

					let org_id = org_id.unwrap();
					let organisation_id = hex::decode(&org_id);
					if organisation_id.is_err() {
						context.status(400).json(error!(WRONG_PARAMETERS));
						return Ok((context, None));
					}
					let organisation_id = organisation_id.unwrap();

					let resource = db::get_resource_by_id(
						context.get_mysql_connection(),
						&organisation_id,
					)
					.await
					.status(500)
					.body(error!(SERVER_ERROR).to_string())?;
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
		"/:tunnelId/get-bash-script",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::portus::VIEW,
				api_macros::closure_as_pinned_box!(|mut context| {
					let tunnel_id = context.get_param(request_keys::TUNNEL_ID);

					if tunnel_id.is_none() {
						context.status(400).json(error!(WRONG_PARAMETERS));
						return Ok((context, None));
					}
					let tunnel_id = tunnel_id.unwrap();

					let tunnel_id = hex::decode(&tunnel_id);
					if tunnel_id.is_err() {
						context.status(400).json(error!(WRONG_PARAMETERS));
						return Ok((context, None));
					}
					let tunnel_id = tunnel_id.unwrap();

					let resource = db::get_resource_by_id(
						context.get_mysql_connection(),
						&tunnel_id,
					)
					.await
					.status(500)
					.body(error!(SERVER_ERROR).to_string())?;

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

	sub_app.get(
		"/:tunnelId/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::portus::VIEW,
				api_macros::closure_as_pinned_box!(|mut context| {
					let tunnel_id = context.get_param(request_keys::TUNNEL_ID);

					if tunnel_id.is_none() {
						context.status(400).json(error!(WRONG_PARAMETERS));
						return Ok((context, None));
					}
					let tunnel_id = tunnel_id.unwrap();

					let tunnel_id = hex::decode(&tunnel_id);
					if tunnel_id.is_err() {
						context.status(400).json(error!(WRONG_PARAMETERS));
						return Ok((context, None));
					}
					let tunnel_id = tunnel_id.unwrap();

					let resource = db::get_resource_by_id(
						context.get_mysql_connection(),
						&tunnel_id,
					)
					.await
					.status(500)
					.body(error!(SERVER_ERROR).to_string())?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(get_info_for_tunnel)),
		],
	);

	sub_app.delete(
		"/:tunnelId/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::portus::DELETE,
				api_macros::closure_as_pinned_box!(|mut context| {
					let tunnel_id = context.get_param(request_keys::TUNNEL_ID);

					if tunnel_id.is_none() {
						context.status(400).json(error!(WRONG_PARAMETERS));
						return Ok((context, None));
					}
					let tunnel_id = tunnel_id.unwrap();

					let tunnel_id = hex::decode(&tunnel_id);
					if tunnel_id.is_err() {
						context.status(400).json(error!(WRONG_PARAMETERS));
						return Ok((context, None));
					}
					let tunnel_id = tunnel_id.unwrap();

					let resource = db::get_resource_by_id(
						context.get_mysql_connection(),
						&tunnel_id,
					)
					.await
					.status(500)
					.body(error!(SERVER_ERROR).to_string())?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(delete_tunnel)),
		],
	);

	sub_app
}

async fn get_tunnels_for_organisation(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let organisation_id =
		hex::decode(context.get_param(request_keys::ORGANISATION_ID).unwrap())
			.unwrap();

	let tunnels = db::get_portus_tunnels_for_organisation(
		context.get_mysql_connection(),
		&organisation_id,
	)
	.await?
	.into_iter()
	.map(|tunnel| {
		let id = hex::encode(tunnel.id);
		json!({
			request_keys::ID: id,
			request_keys::USERNAME: tunnel.username,
			request_keys::SSH_PORT: tunnel.ssh_port,
			request_keys::EXPOSED_PORT: tunnel.exposed_port,
			request_keys::CREATED: tunnel.created,
			request_keys::NAME: tunnel.name,
			request_keys::SERVER_IP: service::get_server_ip_address(),
		})
	})
	.collect::<Vec<_>>();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::TUNNELS: tunnels,
	}));

	Ok(context)
}

// fn to get information for given tunnel/tunnelId
async fn get_info_for_tunnel(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	// get tunnel id from parameter
	// since tunnel id will already ge authenticated in resource token
	// authenticator, we can safely unwrap tunnel id.
	let tunnel_id_string =
		context.get_param(request_keys::TUNNEL_ID).unwrap().clone();
	let tunnel_id = hex::decode(&tunnel_id_string).unwrap();

	//  get tunnel info  using tunnel id
	let tunnel = db::get_portus_tunnel_by_tunnel_id(
		context.get_mysql_connection(),
		&tunnel_id,
	)
	.await?;
	if tunnel.is_none() {
		context.status(404).json(error!(RESOURCE_DOES_NOT_EXIST));
		return Ok(context);
	}
	let tunnel = tunnel.unwrap();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::TUNNEL_ID: tunnel_id_string,
		request_keys::USERNAME: tunnel.username,
		request_keys::SSH_PORT: tunnel.ssh_port,
		request_keys::EXPOSED_PORT: tunnel.exposed_port,
		request_keys::CREATED: tunnel.created,
		request_keys::NAME: tunnel.name,
		request_keys::SERVER_IP: service::get_server_ip_address()
	}));

	Ok(context)
}

// fn to delete tunnel by tunnelId
async fn delete_tunnel(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	// get tunnel id from parameter
	// since tunnel id will already get authenticated in resource token
	// authenticator, we can safely unwrap tunnel id.
	let tunnel_id = context.get_param(request_keys::TUNNEL_ID).unwrap();
	let tunnel_id = &hex::decode(&tunnel_id).unwrap();

	//  get tunnel info  using tunnel id
	let tunnel = db::get_portus_tunnel_by_tunnel_id(
		context.get_mysql_connection(),
		&tunnel_id,
	)
	.await?;
	if tunnel.is_none() {
		context.status(404).json(error!(RESOURCE_DOES_NOT_EXIST));
		return Ok(context);
	}
	let tunnel = tunnel.unwrap();

	let docker = Docker::new();
	let container_name = service::get_container_name(&tunnel.username);

	db::delete_portus_tunnel(context.get_mysql_connection(), &tunnel_id)
		.await
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let container_delete_result =
		service::delete_container(&docker, &container_name).await;

	if let Err(err) = container_delete_result {
		let err_message =
			format!("Error while deleting the container: {:?}", err);
		return Err(Error::new(Box::new(err)).status(500).body(err_message));
	}

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

async fn create(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();
	// get tunnel name
	let tunnel_name = if let Some(Value::String(tunnel_name)) =
		body.get(request_keys::NAME)
	{
		tunnel_name
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	// get user name and user id from tokendata
	let username = service::generate_username(10);
	let organisation_id =
		context.get_param(request_keys::ORGANISATION_ID).unwrap();
	let organisation_id = hex::decode(&organisation_id).unwrap();

	// generate new resource id for the generated container.
	let resource_id =
		db::generate_new_resource_id(context.get_mysql_connection())
			.await
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	let resource_id = resource_id.as_bytes(); // convert to byte array

	// generate unique password
	let generated_password = service::generate_password(10);

	// create container
	let docker = Docker::new();
	let image = constants::PORTUS_DOCKER_IMAGE;
	let container_name = service::get_container_name(username.as_str());
	let ssh_port =
		service::assign_available_port(context.get_mysql_connection())
			.await
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	let exposed_port =
		service::assign_available_port(context.get_mysql_connection())
			.await
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	let image_ssh_port = service::get_ssh_port_for_server();
	let server_ip_address = service::get_server_ip_address();
	let created = get_current_time();

	// check if container name already exists
	let portus_tunnel = db::get_portus_tunnel_by_name(
		context.get_mysql_connection(),
		&tunnel_name,
	)
	.await?;

	if portus_tunnel.is_some() {
		log::info!("Container with the name already exists");
		context.status(500).json(error!(RESOURCE_EXISTS));
		return Ok(context);
	}

	let container_info = docker
		.containers()
		.create(
			&ContainerOptions::builder(image)
				.name(&container_name)
				.env(vec![
					format!("SUDO_ACCESS={}", true).as_str(),
					format!("PASSWORD_ACCESS={}", true).as_str(),
					format!("USER_PASSWORD={}", &generated_password).as_str(),
					format!("USER_NAME={}", &username).as_str(),
				])
				.expose(image_ssh_port, "tcp", ssh_port)
				.expose(exposed_port, "tcp", exposed_port)
				.build(),
		)
		.await
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let container_id = container_info.id;

	let container_start_result =
		service::start_container(&docker, &container_id).await;
	// create resource in db
	let create_resource_result = db::create_resource(
		context.get_mysql_connection(),
		resource_id,
		&format!("portus-tunnel-{}", tunnel_name),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::PORTUS)
			.unwrap(),
		&organisation_id,
	)
	.await;

	let create_tunnel_result = db::create_new_portus_tunnel(
		context.get_mysql_connection(),
		resource_id,
		&username,
		ssh_port,
		exposed_port,
		&tunnel_name,
		created,
	)
	.await;

	//  add container information in portus table
	type StdErrorType = Box<dyn StdError + Send + Sync + 'static>;
	let tunnel_register_result = container_start_result
		.map_err::<StdErrorType, _>(|err| Box::new(err))
		.and(
			create_resource_result
				.map_err::<StdErrorType, _>(|err| Box::new(err)),
		)
		.and(
			create_tunnel_result
				.map_err::<StdErrorType, _>(|err| Box::new(err)),
		);
	if let Err(err) = tunnel_register_result {
		log::error!("Error while adding data to portus table. {:#?}", err);

		// if there is an error in database query, stop the container which just
		// started.
		log::info!("Stopping and Deleting container {} ...", &container_name);
		let container_delete_result =
			service::delete_container(&docker, &container_name).await;
		if let Err(err) = container_delete_result {
			log::error!("could not delete container. Error => {:#?}", err);
		}
		log::error!("could not delete container. Error => {:#?}", err);

		context.status(500).json(error!(SERVER_ERROR));
		let err_message =
			format!("Error creating container for portus tunnel: {:?}", err);
		return Err(Error::new(err).status(500).body(err_message));
	};

	// on success, return ssh port, username,  exposed port, server ip address,
	// password
	let resource_id = hex::encode(resource_id);
	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ID: resource_id,
		request_keys::NAME: &tunnel_name,
		request_keys::USERNAME: &username,
		request_keys::PASSWORD: &generated_password,
		request_keys::SERVER_IP: &server_ip_address,
		request_keys::SSH_PORT: ssh_port,
		request_keys::CREATED: created,
		request_keys::EXPOSED_PORT: vec![exposed_port],
	}));
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
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
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

	let exposed_server_port = if let Some(Value::String(exposed_server_port)) =
		body.get(request_keys::EXPOSED_SERVER_PORT)
	{
		exposed_server_port
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let server_ip_address = if let Some(Value::String(server_ip_address)) =
		body.get(request_keys::SERVER_IP)
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

	let server_ssh_port = if let Some(Value::String(server_ssh_port)) =
		body.get(request_keys::SERVER_SSH_PORT)
	{
		server_ssh_port
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let bash_script_file_content = service::bash_script_formatter(
		local_port,
		local_host_name,
		exposed_server_port,
		server_ip_address,
		server_user_name,
		server_ssh_port,
	)
	.await?;

	// set attachment type
	// context.attachment(Some(&"connect-pi-to-server.sh".to_owned()[..]));
	// context.body(&bash_script_file_content[..]);
	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::SCRIPT: bash_script_file_content,
	}));
	Ok(context)
}
