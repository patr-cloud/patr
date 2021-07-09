use std::error::Error as StdError;

use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use hex::ToHex;
use serde_json::json;
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
		get_current_time_millis,
		Error,
		ErrorData,
		EveContext,
		EveMiddleware,
	},
};
type StdErrorType = Box<dyn StdError + Send + Sync + 'static>;

/// # Description
/// This function is used to create a sub app for every endpoint listed. It
/// creates an eve app which binds the endpoint with functions.
///
/// # Arguments
/// * `app` - an object of type [`App`] which contains all the configuration of
///   api including the
/// database connections.
///
/// # Returns
/// this function returns `EveApp<EveContext, EveMiddleware, App, ErrorData>`
/// containing context, middleware, object of [`App`] and Error
///
/// [`App`]: App
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
					let org_id = context
						.get_param(request_keys::ORGANISATION_ID)
						.unwrap();
					let organisation_id = hex::decode(&org_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
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
					let org_id = context
						.get_param(request_keys::ORGANISATION_ID)
						.unwrap();
					let organisation_id = hex::decode(&org_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
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
		"/:tunnelId/get-bash-script",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::portus::VIEW,
				api_macros::closure_as_pinned_box!(|mut context| {
					let tunnel_id =
						context.get_param(request_keys::TUNNEL_ID).unwrap();

					let tunnel_id = hex::decode(&tunnel_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&tunnel_id,
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

	sub_app.get(
		"/:tunnelId/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::portus::VIEW,
				api_macros::closure_as_pinned_box!(|mut context| {
					let tunnel_id =
						context.get_param(request_keys::TUNNEL_ID).unwrap();

					let tunnel_id = hex::decode(&tunnel_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&tunnel_id,
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
			EveMiddleware::CustomFunction(pin_fn!(get_info_for_tunnel)),
		],
	);

	sub_app.delete(
		"/:tunnelId/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::portus::DELETE,
				api_macros::closure_as_pinned_box!(|mut context| {
					let tunnel_id =
						context.get_param(request_keys::TUNNEL_ID).unwrap();

					let tunnel_id = hex::decode(&tunnel_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&tunnel_id,
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
			EveMiddleware::CustomFunction(pin_fn!(delete_tunnel)),
		],
	);

	sub_app
}

/// # Description
/// This function is used to get list of tunnels for the organisation
/// required inputs:
/// auth token in the headers
/// organisation id in the url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false,
///    tunnels:
///    [
///       {
///          id: ,
///          username: ,
///          sshPort: ,
///          exposedPort: ,
///          created: ,
///          name: ,
///          serverIp:
///       }
///    ]
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_tunnels_for_organisation(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let organisation_id =
		hex::decode(context.get_param(request_keys::ORGANISATION_ID).unwrap())
			.unwrap();

	let tunnels = db::get_portus_tunnels_for_organisation(
		context.get_database_connection(),
		&organisation_id,
	)
	.await?
	.into_iter()
	.map(|tunnel| {
		let id = tunnel.id.encode_hex::<String>();
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

/// # Description
/// This function is used to get information for given tunnel/tunnelId
/// required inputs:
/// auth token in the headers
/// organisation id in the url
/// ```
/// {
///     tunnelId:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false,
///    tunnelId: ,
///    username: ,
///    sshPort: ,
///    exposedPort: ,
///    created: ,
///    name: ,
///    serverIp:
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
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
		context.get_database_connection(),
		&tunnel_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

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

/// # Description
/// This function is used to to delete tunnel by tunnelId
/// required inputs:
/// auth token in the headers
/// organisation id in the url
/// ```
/// {
///    tunnelId:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn delete_tunnel(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	// get tunnel id from parameter
	// since tunnel id will already get authenticated in resource token
	// authenticator, we can safely unwrap tunnel id.
	let tunnel_id = context.get_param(request_keys::TUNNEL_ID).unwrap();
	let tunnel_id = hex::decode(&tunnel_id).unwrap();

	//  get tunnel info  using tunnel id
	let tunnel = db::get_portus_tunnel_by_tunnel_id(
		context.get_database_connection(),
		&tunnel_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let docker = Docker::new();
	let container_name = service::get_container_name(&tunnel.username);

	db::delete_portus_tunnel(context.get_database_connection(), &tunnel_id)
		.await?;

	service::delete_container(&docker, &container_name).await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

/// # Description
/// This function is used to create a new tunnel
/// required inputs:
/// auth token in the headers
/// organisation id in the url
/// ```
/// {
///     name:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false,
///    id: ,
///    name: ,
///    username: ,
///    password: ,
///    serverIp: ,
///    sshPort: ,
///    created: ,
///    exposedPort:
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn create(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();
	// get tunnel name
	let tunnel_name = body
		.get(request_keys::NAME)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	// get user name and user id from tokendata
	let username = service::generate_username(10);
	let organisation_id =
		context.get_param(request_keys::ORGANISATION_ID).unwrap();
	let organisation_id = hex::decode(&organisation_id).unwrap();

	// generate new resource id for the generated container.
	let resource_id =
		db::generate_new_resource_id(context.get_database_connection()).await?;
	let resource_id = resource_id.as_bytes(); // convert to byte array

	// generate unique password
	let generated_password = service::generate_password(10);

	// create container
	let docker = Docker::new();
	let image = constants::PORTUS_DOCKER_IMAGE;
	let container_name = service::get_container_name(username.as_str());
	let ssh_port =
		service::assign_available_port(context.get_database_connection())
			.await?;
	let exposed_port =
		service::assign_available_port(context.get_database_connection())
			.await?;
	let image_ssh_port = service::get_ssh_port_for_server();
	let server_ip_address = service::get_server_ip_address();
	let created = get_current_time_millis();

	// check if container name already exists
	let portus_tunnel = db::get_portus_tunnel_by_name(
		context.get_database_connection(),
		&tunnel_name,
	)
	.await?;

	if portus_tunnel.is_some() {
		Error::as_result()
			.status(500)
			.body(error!(RESOURCE_EXISTS).to_string())?;
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
				.expose(image_ssh_port.into(), "tcp", ssh_port.into())
				.expose(exposed_port.into(), "tcp", exposed_port.into())
				.build(),
		)
		.await?;

	let container_id = container_info.id;

	let container_start_result =
		service::start_container(&docker, &container_id).await;
	// create resource in db
	let create_resource_result = db::create_resource(
		context.get_database_connection(),
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
		context.get_database_connection(),
		resource_id,
		&username,
		ssh_port,
		exposed_port,
		&tunnel_name,
		created,
	)
	.await;

	//  add container information in portus table
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

		Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	}

	// on success, return ssh port, username,  exposed port, server ip address,
	// password
	let resource_id = resource_id.encode_hex::<String>();
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

/// # Description
/// This function is used to get the content for the bash script
/// required inputs:
/// auth token in the headers
/// organisation id in the url
/// ```
/// {
///      localPort: ,
/// 	 localHostName: ,
/// 	 exposedServerPort: ,
/// 	 serverSSHPort: ,
/// 	 serverIPAddress: ,
/// 	 serverUserName:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///      success: true or false,
///      localPort: , -> port number
///      localHostName: , -> hostname
///      exposedServerPort: , -> exposed port number
///      serverSSHPort: , -> server ssh port number
///      serverIPAddress: , -> server ip address
///      serverUserName:  -> server username
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_bash_script(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	// get request body
	let body = context.get_body_object().clone();
	let local_port = body
		.get(request_keys::LOCAL_PORT)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let local_host_name = body
		.get(request_keys::LOCAL_HOST_NAME)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let exposed_server_port = body
		.get(request_keys::EXPOSED_SERVER_PORT)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let server_ip_address = body
		.get(request_keys::SERVER_IP)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let server_user_name = body
		.get(request_keys::SERVER_USER_NAME)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let server_ssh_port = body
		.get(request_keys::SERVER_SSH_PORT)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

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
