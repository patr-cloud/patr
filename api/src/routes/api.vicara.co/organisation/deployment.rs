use api_macros::closure_as_pinned_box;
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use hex::ToHex;
use serde_json::{json, Value};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac::{self, permissions},
	pin_fn,
	utils::{
		constants::request_keys,
		Error,
		ErrorData,
		EveContext,
		EveMiddleware,
	},
};

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
			request_keys::DOMAIN_ID: deployment.domain_id.encode_hex::<String>(),
			request_keys::SUB_DOMAIN: deployment.sub_domain,
			request_keys::PATH: deployment.path,
			request_keys::PORT: deployment.port,
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

	let image_name = body
		.get(request_keys::IMAGE_NAME)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let image_tag = body
		.get(request_keys::IMAGE_TAG)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let domain_id = body
		.get(request_keys::DOMAIN_ID)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let sub_domain = body
		.get(request_keys::SUB_DOMAIN)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let path = body
		.get(request_keys::PATH)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let port = body
		.get(request_keys::NAME)
		.map(|value| match value {
			Value::Number(number) => {
				if number.is_u64() {
					Some(number.as_u64().unwrap() as u16)
				} else if number.is_i64() {
					Some(number.as_i64().unwrap() as u16)
				} else {
					None
				}
			}
			Value::String(value) => value.parse::<u16>().ok(),
			_ => None,
		})
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

	let domain_id = hex::decode(domain_id)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let deployment = db::get_deployment_by_entry_point(
		context.get_mysql_connection(),
		&domain_id,
		sub_domain,
		path,
	)
	.await?;
	if deployment.is_some() {
		Error::as_result()
			.status(404)
			.body(error!(RESOURCE_EXISTS).to_string())?;
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
	db::create_deployment(
		context.get_mysql_connection(),
		deployment_id,
		name,
		registry,
		image_name,
		image_tag,
		&domain_id,
		sub_domain,
		path,
		port,
	)
	.await?;

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
			request_keys::DOMAIN_ID: deployment.domain_id.encode_hex::<String>(),
			request_keys::SUB_DOMAIN: deployment.sub_domain,
			request_keys::PATH: deployment.path,
			request_keys::PORT: deployment.port,
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
