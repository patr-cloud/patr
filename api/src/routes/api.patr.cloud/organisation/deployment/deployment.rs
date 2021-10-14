use api_macros::closure_as_pinned_box;
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use serde_json::{json, Map, Value};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::{db_mapping::DeploymentStatus, rbac::permissions},
	pin_fn,
	service,
	utils::{
		constants::request_keys,
		validator,
		Error,
		ErrorData,
		EveContext,
		EveMiddleware,
	},
};

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
pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut app = create_eve_app(app);

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

	// start a deployment
	app.post(
		"/:deploymentId/start",
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
			EveMiddleware::CustomFunction(pin_fn!(start_deployment)),
		],
	);

	// stop and delete the deployment
	app.post(
		"/:deploymentId/stop",
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
			EveMiddleware::CustomFunction(pin_fn!(stop_deployment)),
		],
	);

	// get logs for the deployment
	app.get(
		"/:deploymentId/logs",
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
			EveMiddleware::CustomFunction(pin_fn!(get_logs)),
		],
	);

	// get list of environment variables for deployment
	app.get(
		"/:deploymentId/environment-variables",
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
			EveMiddleware::CustomFunction(pin_fn!(get_environment_variables)),
		],
	);

	// set environment variables for deployment
	app.put(
		"/:deploymentId/environment-variables",
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
			EveMiddleware::CustomFunction(pin_fn!(set_environment_variables)),
		],
	);

	// set horizontal scale for the deployment
	app.put(
		"/:deploymentId/horizontal-scale",
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
			EveMiddleware::CustomFunction(pin_fn!(set_horizontal_scale)),
		],
	);

	// set machine type of the deployment
	app.put(
		"/:deploymentId/machine-type",
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
			EveMiddleware::CustomFunction(pin_fn!(set_machine_type)),
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

	// get domain cname and value of deployment
	app.get(
		"/:deploymentId/domain-dns-records",
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
			EveMiddleware::CustomFunction(pin_fn!(get_domain_dns_records)),
		],
	);

	// update domain in the deployment
	app.put(
		"/:deploymentId/domain",
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
			EveMiddleware::CustomFunction(pin_fn!(set_domain_name)),
		],
	);

	// get deployment validation status
	app.get(
		"/:deploymentId/domain-validated",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::deployment::INFO,
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
			EveMiddleware::CustomFunction(pin_fn!(is_domain_validated)),
		],
	);

	// get data center recommendation on the basis of distance
	app.get(
		"/:deploymentId/recommended-data-center",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::deployment::INFO,
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
			EveMiddleware::CustomFunction(pin_fn!(get_recommended_data_center)),
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
		let mut map = Map::new();

		map.insert(request_keys::SUCCESS.to_string(), Value::Bool(true));
		if deployment.registry == "registry.patr.cloud" {
			map.insert(
				request_keys::REPOSITORY_ID.to_string(),
				Value::String(hex::encode(deployment.repository_id?)),
			);
		} else {
			map.insert(
				request_keys::IMAGE_NAME.to_string(),
				Value::String(deployment.image_name?),
			);
		}
		if let Some(domain_name) = deployment.domain_name {
			map.insert(
				request_keys::DOMAIN_NAME.to_string(),
				Value::String(domain_name),
			);
		}
		map.insert(
			request_keys::DEPLOYMENT_ID.to_string(),
			Value::String(hex::encode(deployment.id)),
		);
		map.insert(
			request_keys::NAME.to_string(),
			Value::String(deployment.name),
		);
		map.insert(
			request_keys::REGISTRY.to_string(),
			Value::String(deployment.registry),
		);
		map.insert(
			request_keys::IMAGE_TAG.to_string(),
			Value::String(deployment.image_tag),
		);
		map.insert(
			request_keys::STATUS.to_string(),
			Value::String(deployment.status.to_string()),
		);
		map.insert(
			request_keys::REGION.to_string(),
			Value::String(deployment.region),
		);
		map.insert(
			request_keys::HORIZONTAL_SCALE.to_string(),
			Value::Number(deployment.horizontal_scale.into()),
		);
		map.insert(
			request_keys::MACHINE_TYPE.to_string(),
			Value::String(deployment.machine_type.to_string()),
		);
		Some(Value::Object(map))
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
///    name: ,
///    registry: ,
///    repositoryId: ,
///    imageName: ,
///    imageTag: ,
///    region: ,
///    domainName: ,
///    horizontalScale: ,
///    machineType:
/// }
/// ```
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

	let region = body
		.get(request_keys::REGION)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let domain_name = body
		.get(request_keys::DOMAIN_NAME)
		.map(|value| {
			value
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;
	let horizontal_scale = body
		.get(request_keys::HORIZONTAL_SCALE)
		.map(|value| match value {
			Value::Number(number) => {
				if number.is_u64() {
					number.as_u64()
				} else if number.is_i64() {
					number
						.as_i64()
						.map(|number| {
							if number > 0 {
								Some(number as u64)
							} else {
								None
							}
						})
						.flatten()
				} else {
					None
				}
			}
			Value::String(number) => number.parse::<u64>().ok(),
			_ => None,
		})
		.flatten()
		.map(|number| {
			if number > 0 && number < 256 {
				Some(number)
			} else {
				None
			}
		})
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let machine_type = body
		.get(request_keys::MACHINE_TYPE)
		.map(|value| value.as_str())
		.flatten()
		.map(|machine_type| machine_type.parse().ok())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let config = context.get_state().config.clone();

	let deployment_id = service::create_deployment_in_organisation(
		context.get_database_connection(),
		&organisation_id,
		name,
		registry,
		repository_id,
		image_name,
		image_tag,
		region,
		domain_name,
		horizontal_scale,
		&machine_type,
		&config,
	)
	.await?;

	context.commit_database_transaction().await?;

	// Deploy the app as soon as it's created, so that any existing images can
	// be deployed
	service::start_deployment(
		context.get_database_connection(),
		deployment_id.as_bytes(),
		&config,
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
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// DeploymentId in url
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
///     success: true or false,
///     deployment:
///     {
///         id: ,
///         name: ,
///         registry: ,
///         imageName: ,
///         imageTag: ,
///         domainName: ,
///         horizontalScale: ,
///         machineType:
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

	let mut response = Map::new();

	response.insert(request_keys::SUCCESS.to_string(), Value::Bool(true));
	if deployment.registry == "registry.patr.cloud" {
		response.insert(
			request_keys::REPOSITORY_ID.to_string(),
			Value::String(hex::encode(deployment.repository_id.status(500)?)),
		);
	} else {
		response.insert(
			request_keys::IMAGE_NAME.to_string(),
			Value::String(deployment.image_name.status(500)?),
		);
	}
	if let Some(domain_name) = deployment.domain_name {
		response.insert(
			request_keys::DOMAIN_NAME.to_string(),
			Value::String(domain_name),
		);
	}
	response.insert(
		request_keys::DEPLOYMENT_ID.to_string(),
		Value::String(hex::encode(deployment.id)),
	);
	response.insert(
		request_keys::NAME.to_string(),
		Value::String(deployment.name),
	);
	response.insert(
		request_keys::REGISTRY.to_string(),
		Value::String(deployment.registry),
	);
	response.insert(
		request_keys::IMAGE_TAG.to_string(),
		Value::String(deployment.image_tag),
	);
	response.insert(
		request_keys::STATUS.to_string(),
		Value::String(deployment.status.to_string()),
	);
	response.insert(
		request_keys::REGION.to_string(),
		Value::String(deployment.region),
	);
	response.insert(
		request_keys::HORIZONTAL_SCALE.to_string(),
		Value::Number(deployment.horizontal_scale.into()),
	);
	response.insert(
		request_keys::MACHINE_TYPE.to_string(),
		Value::String(deployment.machine_type.to_string()),
	);

	context.json(Value::Object(response));
	Ok(context)
}

/// # Description
/// This function is used to start a deployment
/// required inputs:
/// deploymentId in the url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the next
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
async fn start_deployment(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let deployment_id =
		hex::decode(context.get_param(request_keys::DEPLOYMENT_ID).unwrap())
			.unwrap();

	// start the container running the image, if doesn't exist
	let config = context.get_state().config.clone();
	service::start_deployment(
		context.get_database_connection(),
		&deployment_id,
		&config,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

/// # Description
/// This function is used to stop a deployment
/// required inputs:
/// deploymentId in the url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the next
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
async fn stop_deployment(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let deployment_id =
		hex::decode(context.get_param(request_keys::DEPLOYMENT_ID).unwrap())
			.unwrap();

	// stop the running container, if it exists
	let config = context.get_state().config.clone();
	service::stop_deployment(
		context.get_database_connection(),
		&deployment_id,
		&config,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

/// # Description
/// This function is used to get the logs of a deployment
/// required inputs:
/// deploymentId in the url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the next
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
async fn get_logs(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let deployment_id =
		hex::decode(context.get_param(request_keys::DEPLOYMENT_ID).unwrap())
			.unwrap();

	// stop the running container, if it exists
	let config = context.get_state().config.clone();
	let logs = service::get_deployment_container_logs(
		context.get_database_connection(),
		&deployment_id,
		&config,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::LOGS: logs,
	}));
	Ok(context)
}

/// # Description
/// This function is used to delete deployment
/// required inputs:
/// deploymentId in the url
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
async fn delete_deployment(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let deployment_id =
		hex::decode(context.get_param(request_keys::DEPLOYMENT_ID).unwrap())
			.unwrap();

	// stop and delete the container running the image, if it exists
	let config = context.get_state().config.clone();
	service::stop_deployment(
		context.get_database_connection(),
		&deployment_id,
		&config,
	)
	.await?;

	db::update_deployment_status(
		context.get_database_connection(),
		&deployment_id,
		&DeploymentStatus::Deleted,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

/// # Description
/// This function is used to get the environment variables for a deployment
/// required inputs:
/// deploymentId in the url
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
async fn get_environment_variables(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let deployment_id =
		hex::decode(context.get_param(request_keys::DEPLOYMENT_ID).unwrap())
			.unwrap();

	let env_vars: Map<String, Value> =
		db::get_environment_variables_for_deployment(
			context.get_database_connection(),
			&deployment_id,
		)
		.await?
		.into_iter()
		.map(|(key, value)| (key, Value::String(value)))
		.collect();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ENVIRONMENT_VARIABLES: env_vars
	}));
	Ok(context)
}

/// # Description
/// This function is used to set the environment variables for a deployment
/// required inputs:
/// deploymentId in the url
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
async fn set_environment_variables(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let deployment_id =
		hex::decode(context.get_param(request_keys::DEPLOYMENT_ID).unwrap())
			.unwrap();
	let body = context.get_body_object().clone();

	let env_var_values = body
		.get(request_keys::ENVIRONMENT_VARIABLES)
		.map(|values| values.as_object())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let mut environment_variables = vec![];

	for (key, value) in env_var_values {
		let value = value
			.as_str()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?;

		environment_variables.push((key.clone(), value.to_string()));
	}

	service::set_environment_variables_for_deployment(
		context.get_database_connection(),
		&deployment_id,
		&environment_variables,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

/// # Description
/// This function is used to set the horizontal scale for a deployment.
/// Deployments need to be restarted before the changes are applied
/// required inputs:
/// deploymentId in the url
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
async fn set_horizontal_scale(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let deployment_id =
		hex::decode(context.get_param(request_keys::DEPLOYMENT_ID).unwrap())
			.unwrap();
	let body = context.get_body_object().clone();

	let horizontal_scale = body
		.get(request_keys::HORIZONTAL_SCALE)
		.map(|value| match value {
			Value::Number(number) => {
				if number.is_u64() {
					number.as_u64()
				} else if number.is_i64() {
					number
						.as_i64()
						.map(|number| {
							if number > 0 {
								Some(number as u64)
							} else {
								None
							}
						})
						.flatten()
				} else {
					None
				}
			}
			Value::String(number) => number.parse::<u64>().ok(),
			_ => None,
		})
		.flatten()
		.map(|number| {
			if number > 0 && number < 256 {
				Some(number)
			} else {
				None
			}
		})
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	db::set_horizontal_scale_for_deployment(
		context.get_database_connection(),
		&deployment_id,
		horizontal_scale,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

/// # Description
/// This function is used to set the machine type for a deployment.
/// Deployments need to be restarted before the changes are applied
/// required inputs:
/// deploymentId in the url
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
async fn set_machine_type(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let deployment_id =
		hex::decode(context.get_param(request_keys::DEPLOYMENT_ID).unwrap())
			.unwrap();
	let body = context.get_body_object().clone();

	let machine_type = body
		.get(request_keys::MACHINE_TYPE)
		.map(|value| value.as_str())
		.flatten()
		.map(|machine_type| machine_type.parse().ok())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	db::set_machine_type_for_deployment(
		context.get_database_connection(),
		&deployment_id,
		&machine_type,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

/// # Description
/// This function is used to get the DNS records for the domain
/// required inputs:
/// deploymentId in the url
/// ```
/// {
///     domainName:
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
///    cnameRecords: [
///         {
///           cname: "domain_name",
///           value: "provider's url"
///         }
///    ]
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_domain_dns_records(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let deployment_id =
		hex::decode(context.get_param(request_keys::DEPLOYMENT_ID).unwrap())
			.unwrap();

	let cname_records = service::get_dns_records_for_deployments(
		context.get_database_connection(),
		&deployment_id,
	)
	.await?
	.into_iter()
	.map(|record| {
		json!({
			request_keys::CNAME: record.cname,
			request_keys::VALUE: record.value
		})
	})
	.collect::<Vec<_>>();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::CNAME_RECORDS: cname_records
	}));
	Ok(context)
}

/// # Description
/// This function is used to set the domain name of the deployment
/// required inputs:
/// deploymentId in the url
/// ```
/// {
///     domainName:
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
///    cnameRecords: [
///         {
///           cname: "domain_name",
///           value: "provider's url"
///         }
///    ]
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn set_domain_name(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let deployment_id =
		hex::decode(context.get_param(request_keys::DEPLOYMENT_ID).unwrap())
			.unwrap();

	let body = context.get_body_object().clone();
	let domain_name = body
		.get(request_keys::DOMAIN_NAME)
		.map(|value| {
			value
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	if let Some(domain_name) = domain_name {
		if !validator::is_deployment_entry_point_valid(domain_name) {
			return Err(Error::empty()
				.status(400)
				.body(error!(INVALID_DOMAIN_NAME).to_string()));
		}
	}
	let config = context.get_state().config.clone();

	service::set_domain_for_deployment(
		context.get_database_connection(),
		&config,
		&deployment_id,
		domain_name,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

/// # Description
/// This function is used to get the status of domain set for deployment
/// required inputs:
/// deploymentId in the url
/// ```
/// {
///     domainName:
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
async fn is_domain_validated(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let deployment_id =
		hex::decode(context.get_param(request_keys::DEPLOYMENT_ID).unwrap())
			.unwrap();
	let config = context.get_state().config.clone();

	let validated = service::get_domain_validation_status(
		context.get_database_connection(),
		&deployment_id,
		&config,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::VALIDATED: validated,
	}));
	Ok(context)
}

/// # Description
/// This function is used to get the nearest data center for the majority of the
/// users deploymentId in the url
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
///    datacenters: []
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_recommended_data_center(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let deployment_id =
		hex::decode(context.get_param(request_keys::DEPLOYMENT_ID).unwrap())
			.unwrap();

	let data_center = db::get_recommended_data_center(
		context.get_database_connection(),
		&deployment_id,
	)
	.await?
	.status(500)?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::RECOMMENDED_DATA_CENTERS: {
			request_keys::REGION: data_center.region,
			request_keys::DISTANCE: data_center.avg_distance,
		}
	}));
	Ok(context)
}
