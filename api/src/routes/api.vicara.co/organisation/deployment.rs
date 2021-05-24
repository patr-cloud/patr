use std::convert::TryInto;

use api_macros::closure_as_pinned_box;
use chrono::naive::serde;
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use hex::ToHex;
use s3::request;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::{
		db_mapping::{EntryPoint, EnvVariable, VolumeMount},
		rbac::{self, permissions},
	},
	pin_fn,
	utils::{
		constants::request_keys,
		Error,
		ErrorData,
		EveContext,
		EveMiddleware,
	},
};

// TODO: create an end point that spins up new container with upgraded path
// storing config data, and spinning up according to it.
// upgrade path

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
						context.get_mysql_connection(),
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

	// Delete a deployment
	app.get(
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
						context.get_mysql_connection(),
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

	// endpoint to create machine type
	app.post(
		"/machine-type",
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
			EveMiddleware::CustomFunction(pin_fn!(create_machine_type)),
		],
	);

	// endpoint to add in deployment configuration.
	app.post(
		"/:deploymentId/config",
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
			EveMiddleware::CustomFunction(pin_fn!(create_deployment_config)),
		],
	);

	app
}

async fn list_deployments(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let organisation_id =
		hex::decode(context.get_param(request_keys::ORGANISATION_ID).unwrap())
			.unwrap();
	let deployments = db::get_deployments_for_organisation(
		context.get_mysql_connection(),
		&organisation_id,
	)
	.await?
	.into_iter()
	.map(|deployment| {
		json!({
			request_keys::DEPLOYMENT_ID: deployment.id.encode_hex::<String>(),
			request_keys::NAME: deployment.name,
			request_keys::REGISTRY: deployment.registry,
			request_keys::IMAGE_NAME: deployment.image_name,
			request_keys::IMAGE_TAG: deployment.image_tag,
		})
	})
	.collect::<Vec<_>>();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::DEPLOYMENTS: deployments
	}));
	Ok(context)
}

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

	match registry {
		"registry.docker.vicara.co" | "registry.hub.docker.com" => (),
		_ => {
			Error::as_result()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())?;
		}
	}

	let deployment_id =
		db::generate_new_resource_id(context.get_mysql_connection()).await?;
	let deployment_id = deployment_id.as_bytes();

	db::create_resource(
		context.get_mysql_connection(),
		deployment_id,
		&format!("Deployment: {}", name),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::DEPLOYMENT)
			.unwrap(),
		&organisation_id,
	)
	.await?;

	if registry == "registry.docker.vicara.co" && repository_id.is_none() {
		Error::as_result()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?;
	} else if registry != "registry.docker.vicara.co" && image_name.is_none() {
		Error::as_result()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?;
	}
	match registry {
		"registry.docker.vicara.co" => {
			let repository_id = hex::decode(repository_id.unwrap())
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())?;

			db::create_deployment(
				context.get_mysql_connection(),
				deployment_id,
				name,
				registry,
				Some(repository_id),
				None,
				image_tag,
			)
			.await?;
		}
		_ => {
			db::create_deployment(
				context.get_mysql_connection(),
				deployment_id,
				name,
				registry,
				None,
				image_name,
				image_tag,
			)
			.await?;
		}
	}

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::DEPLOYMENT_ID: deployment_id.encode_hex::<String>()
	}));
	Ok(context)
}

async fn get_deployment_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let deployment_id =
		hex::decode(context.get_param(request_keys::DEPLOYMENT_ID).unwrap())
			.unwrap();
	let deployment = db::get_deployment_by_id(
		context.get_mysql_connection(),
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

async fn delete_deployment(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let deployment_id =
		hex::decode(context.get_param(request_keys::DEPLOYMENT_ID).unwrap())
			.unwrap();
	db::get_deployment_by_id(context.get_mysql_connection(), &deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	db::delete_deployment_by_id(context.get_mysql_connection(), &deployment_id)
		.await?;

	// TODO stop and delete the container running the image, if it exists

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

// request body might contain all the config parameters, extract them and add
// them to machine type id| response: `id` of the created machine type
async fn create_machine_type(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let name = body
		.get(request_keys::NAME)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let cpu_count = body
		.get(request_keys::CPU_COUNT)
		.map(|value| value.as_u64())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let memory_count = body
		.get(request_keys::MEMORY_COUNT)
		.map(|value| value.as_f64())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	// convert to f32
	let memory_count = memory_count as f32;

	let gpu_id = body
		.get(request_keys::GPU_ID)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let gpu_id = hex::decode(gpu_id).unwrap();

	// check if the given machine type already exists
	// if yes, get the id from the table. else, add a new machine type
	let is_machine_available = db::get_deployment_machine_type(
		context.get_mysql_connection(),
		&name,
		cpu_count.try_into().unwrap(),
		memory_count,
		&gpu_id,
	)
	.await?;

	// if machine exists, return machine id
	if is_machine_available.is_some() {
		context.json(json!({
			request_keys::SUCCESS : true,
			request_keys::MACHINE_ID: is_machine_available.unwrap().id
		}));
		return Ok(context);
	}

	let machine_type_id =
		db::generate_new_resource_id(context.get_mysql_connection())
			.await?
			.as_bytes()
			.to_vec();

	// function to crate a machine type
	db::create_deployment_machine_type(
		context.get_mysql_connection(),
		&machine_type_id,
		name,
		cpu_count.try_into().unwrap(),
		memory_count,
		&gpu_id,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS : true,
		request_keys::MACHINE_ID: &machine_type_id
	}));
	Ok(context)
}

// function to store port, env variables and mount path
pub async fn create_deployment_config(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let deployment_id =
		hex::decode(context.get_param(request_keys::DEPLOYMENT_ID).unwrap())
			.unwrap();
	let body = context.get_body_object().clone();

	// get array of ports
	let ports = body
		.get(request_keys::PORT)
		.map(|values| values.as_array())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let variable_list = body
		.get(request_keys::VARIABLE_LIST)
		.map(|values| values.as_array())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let volume_list = body
		.get(request_keys::VOLUME_LIST)
		.map(|values| values.as_array())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let entry_point_list = body
		.get(request_keys::ENTRY_POINT_LIST)
		.map(|values| values.as_array())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	// check if deployment exists.
	let deployment = db::get_deployment_by_id(
		context.get_mysql_connection(),
		&deployment_id,
	)
	.await?;

	// throw resource does not exist if deployment is `none`
	if deployment.is_none() {
		context.status(404).json(error!(RESOURCE_DOES_NOT_EXIST));
		return Ok(context);
	}

	// iterate over port array and add it to port table
	for port_value in ports {
		let port = serde_json::from_value(port_value.to_owned())?;
		db::insert_deployment_port(
			context.get_mysql_connection(),
			&deployment_id,
			port,
		)
		.await?;
	}
	// iterate over variable array and add it to variable table
	for variable_value in variable_list {
		let variable: EnvVariable =
			serde_json::from_value(variable_value.to_owned())?;
		db::insert_deployment_environment_variable(
			context.get_mysql_connection(),
			&deployment_id,
			&variable.name,
			&variable.value,
		)
		.await?;
	}
	// iterate over volume array and add it to volume table
	for volume_value in volume_list {
		let volume: VolumeMount =
			serde_json::from_value(volume_value.to_owned())?;
		db::insert_deployment_volumes(
			context.get_mysql_connection(),
			&deployment_id,
			&volume.name,
			&volume.path,
		)
		.await?;
	}

	// iterate over entry point array and add it to `deployment_entry_point`
	// table
	for entry_value in entry_point_list {
		let entry_point: EntryPoint =
			serde_json::from_value(entry_value.to_owned())?;
		db::insert_deployment_entry_point(
			context.get_mysql_connection(),
			&deployment_id,
			&entry_point.domain_id,
			&entry_point.sub_domain,
			&entry_point.path,
		)
		.await?;
	}

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::MESSAGE : "data added successfully"
	}));

	Ok(context)
}
