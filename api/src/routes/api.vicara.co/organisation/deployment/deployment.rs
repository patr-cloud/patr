use api_macros::closure_as_pinned_box;
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use hex::ToHex;
use serde_json::{json, Map, Value};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac::permissions,
	pin_fn,
	service,
	utils::{
		constants::request_keys,
		Error,
		ErrorData,
		EveContext,
		EveMiddleware,
	},
};

/// # Description
/// This function is used to create a sub app for every endpoint listed. It creates an eve app
/// which binds the endpoint with functions.
/// 
/// # Arguments
/// * `app` - an object of type [`App`] which contains all the configuration of api including the
/// database connections.
/// 
/// # Returns
/// this function returns `EveApp<EveContext, EveMiddleware, App, ErrorData>` containing context, middleware, object
/// of [`App`] and Error
/// 
/// [`App`]: App
pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut app = create_eve_app(&app);

	// List all deployments
	app.get(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::deployment::LIST,
				closure_as_pinned_box!(|mut context| {
					let org_id_string = context
						.get_param(request_keys::ORGANISATION_ID)
						.unwrap();
					let organisation_id = hex::decode(&org_id_string)
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
			EveMiddleware::CustomFunction(pin_fn!(list_deployments)),
		],
	);

	// Create a new deployment
	app.post(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::deployment::CREATE,
				closure_as_pinned_box!(|mut context| {
					let org_id_string = context
						.get_param(request_keys::ORGANISATION_ID)
						.unwrap();
					let organisation_id = hex::decode(&org_id_string)
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
			EveMiddleware::CustomFunction(pin_fn!(create_deployment)),
		],
	);

	// Get info about a deployment
	app.get(
		"/:deploymentId/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::deployment::INFO,
				closure_as_pinned_box!(|mut context| {
					let deployment_id_string =
						context.get_param(request_keys::DEPLOYMENT_ID).unwrap();
					let deployment_id = hex::decode(&deployment_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&deployment_id,
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
			EveMiddleware::CustomFunction(pin_fn!(get_deployment_info)),
		],
	);

	// endpoint to update deployment configuration.
	app.get(
		"/:deploymentId/configuration",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::deployment::INFO,
				closure_as_pinned_box!(|mut context| {
					let deployment_id_string =
						context.get_param(request_keys::DEPLOYMENT_ID).unwrap();
					let deployment_id = hex::decode(&deployment_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&deployment_id,
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
			EveMiddleware::CustomFunction(pin_fn!(get_deployment_config)),
		],
	);

	// endpoint to update deployment configuration.
	app.post(
		"/:deploymentId/configuration",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::deployment::EDIT,
				closure_as_pinned_box!(|mut context| {
					let deployment_id_string =
						context.get_param(request_keys::DEPLOYMENT_ID).unwrap();
					let deployment_id = hex::decode(&deployment_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&deployment_id,
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
			EveMiddleware::CustomFunction(pin_fn!(update_deployment_config)),
		],
	);

	// Delete a deployment
	app.delete(
		"/:deploymentId/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::deployment::DELETE,
				closure_as_pinned_box!(|mut context| {
					let deployment_id_string =
						context.get_param(request_keys::DEPLOYMENT_ID).unwrap();
					let deployment_id = hex::decode(&deployment_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&deployment_id,
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
			EveMiddleware::CustomFunction(pin_fn!(delete_deployment)),
		],
	);

	app
}

/// # Description
/// This function is used to list of all the deployments present with the user
/// required inputs: 
/// OrganisationId in url
/// 
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response, database connection, body, 
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the function
/// 
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of [`EveContext`] or an error 
/// output:
/// ```
/// {
///    success:
///    deployments: []
/// }
/// ```
/// 
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn list_deployments(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let organisation_id =
		hex::decode(context.get_param(request_keys::ORGANISATION_ID).unwrap())
			.unwrap();
	let deployments = db::get_deployments_for_organisation(
		context.get_database_connection(),
		&organisation_id,
	)
	.await?
	.into_iter()
	.filter_map(|deployment| {
		if deployment.registry == "registry.docker.vicara.co" {
			Some(json!({
				request_keys::DEPLOYMENT_ID: hex::encode(deployment.id),
				request_keys::NAME: deployment.name,
				request_keys::REGISTRY: deployment.registry,
				request_keys::REPOSITORY_ID: hex::encode(deployment.repository_id?),
				request_keys::IMAGE_TAG: deployment.image_tag,
			}))
		} else {
			Some(json!({
				request_keys::DEPLOYMENT_ID: hex::encode(deployment.id),
				request_keys::NAME: deployment.name,
				request_keys::REGISTRY: deployment.registry,
				request_keys::IMAGE_NAME: deployment.image_name?,
				request_keys::IMAGE_TAG: deployment.image_tag,
			}))
		}
	})
	.collect::<Vec<_>>();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::DEPLOYMENTS: deployments
	}));
	Ok(context)
}

/// # Description
/// This function is used to create a new deployment
/// required inputs
/// auth token in the header
/// organisation id in parameter
/// ```
/// {
///    name:
///    registry:
///    repositoryId:
///    imageName:
///    imageTag:
/// }
/// ```
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response, database connection, body, 
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the function
/// 
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of [`EveContext`] or an error 
/// output:
/// ```
/// {
///    success:
///    deploymentId:
/// }
/// ```
/// 
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn create_deployment(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let organisation_id =
		hex::decode(context.get_param(request_keys::ORGANISATION_ID).unwrap())
			.unwrap();
	let body = context.get_body_object().clone();

	let name = body
		.get(request_keys::NAME)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let registry = body
		.get(request_keys::REGISTRY)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let repository_id = body
		.get(request_keys::REPOSITORY_ID)
		.map(|value| {
			value
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	let image_name = body
		.get(request_keys::IMAGE_NAME)
		.map(|value| {
			value
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	let image_tag = body
		.get(request_keys::IMAGE_TAG)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let deployment_id = service::create_deployment_in_organisation(
		context.get_database_connection(),
		&organisation_id,
		name,
		registry,
		repository_id,
		image_name,
		image_tag,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::DEPLOYMENT_ID: hex::encode(deployment_id.as_bytes())
	}));
	Ok(context)
}

/// # Description
/// This function is used to get the information about a specific deployment
/// required inputs:
/// auth token in the headers
/// DeploymentId in url
/// 
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response, database connection, body, 
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the function
/// 
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of [`EveContext`] or an error 
/// output:
/// ```
/// {
///     success: true or false,
///     deployment:
///     {
///         name: ,
///         registry: ,
///         imageName: ,
///         imageTag: ,
///     }
/// }
/// ```
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_deployment_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let deployment_id =
		hex::decode(context.get_param(request_keys::DEPLOYMENT_ID).unwrap())
			.unwrap();
	let deployment = db::get_deployment_by_id(
		context.get_database_connection(),
		&deployment_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::DEPLOYMENT: {
			request_keys::DEPLOYMENT_ID: deployment.id.encode_hex::<String>(),
			request_keys::NAME: deployment.name,
			request_keys::REGISTRY: deployment.registry,
			request_keys::IMAGE_NAME: deployment.image_name,
			request_keys::IMAGE_TAG: deployment.image_tag,
		}
	}));
	Ok(context)
}

/// # Description
/// This function is used to get the configuration of the deployment
/// required inputs: 
/// deploymentId
/// 
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response, database connection, body, 
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the function
/// 
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of [`EveContext`] or an error 
/// output:
/// ```
/// {
///    success:
///    environmentVariables: []
///    exposedPorts: []
///    persistentVolumes: []
/// }
/// ```
/// 
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_deployment_config(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let deployment_id =
		hex::decode(context.get_param(request_keys::DEPLOYMENT_ID).unwrap())
			.unwrap();
	db::get_deployment_by_id(context.get_database_connection(), &deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let env_vars: Map<String, Value> =
		db::get_environment_variables_for_deployment(
			context.get_database_connection(),
			&deployment_id,
		)
		.await?
		.into_iter()
		.map(|(key, value)| (key, Value::String(value)))
		.collect();
	let ports = db::get_exposed_ports_for_deployment(
		context.get_database_connection(),
		&deployment_id,
	)
	.await?;
	let volumes: Map<String, Value> =
		db::get_persistent_volumes_for_deployment(
			context.get_database_connection(),
			&deployment_id,
		)
		.await?
		.into_iter()
		.map(|volume| (volume.name, Value::String(volume.path)))
		.collect();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ENVIRONMENT_VARIABLES: env_vars,
		request_keys::EXPOSED_PORTS: ports,
		request_keys::PERSISTENT_VOLUMES: volumes
	}));
	Ok(context)
}

// function to store port, env variables and mount path
/// # Description
/// This function is used to store the port, env variables and mount path
/// required inputs:
/// ```
/// {
///    exposedPorts:[]
///    environmentVariables:[]
///    persistentVolumes: []
/// }
/// ```
/// 
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response, database connection, body, 
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the function
/// 
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of [`EveContext`] or an error 
/// output:
/// ```
/// {
///    success: true or false
/// }
/// ```
/// 
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
pub async fn update_deployment_config(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let deployment_id =
		hex::decode(context.get_param(request_keys::DEPLOYMENT_ID).unwrap())
			.unwrap();
	let body = context.get_body_object().clone();

	// get array of ports
	let port_values = body
		.get(request_keys::EXPOSED_PORTS)
		.map(|values| values.as_array())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let env_var_values = body
		.get(request_keys::ENVIRONMENT_VARIABLES)
		.map(|values| values.as_object())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let volume_values = body
		.get(request_keys::PERSISTENT_VOLUMES)
		.map(|values| values.as_object())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let mut exposed_ports = vec![];
	let mut environment_variables = vec![];
	let mut persistent_volumes = vec![];

	for port in port_values {
		let port = serde_json::from_value(port.clone())
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?;
		exposed_ports.push(port);
	}

	for (key, value) in env_var_values {
		let value = value
			.as_str()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?;
		environment_variables.push((key.as_str(), value));
	}

	for (name, path) in volume_values {
		let path = path
			.as_str()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?;
		persistent_volumes.push((name.as_str(), path));
	}

	service::update_configuration_for_deployment(
		context.get_database_connection(),
		&deployment_id,
		&exposed_ports,
		&environment_variables,
		&persistent_volumes,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

/// # Description
/// This function is used to delete deployment
/// required inputs:
/// deploymentId in the url
/// 
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response, database connection, body, 
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the function
/// 
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of [`EveContext`] or an error 
/// output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn delete_deployment(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let deployment_id =
		hex::decode(context.get_param(request_keys::DEPLOYMENT_ID).unwrap())
			.unwrap();
	db::get_deployment_by_id(context.get_database_connection(), &deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	db::delete_deployment_by_id(
		context.get_database_connection(),
		&deployment_id,
	)
	.await?;

	// TODO stop and delete the container running the image, if it exists

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}
