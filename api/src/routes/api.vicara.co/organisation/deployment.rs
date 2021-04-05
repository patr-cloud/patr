use api_macros::closure_as_pinned_box;
use eve_rs::{App as EveApp, Context, Error, NextHandler};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac::{self, permissions},
	pin_fn,
	utils::{constants::request_keys, EveContext, EveMiddleware},
};
use serde_json::{json, Value};

pub fn create_sub_app(app: &App) -> EveApp<EveContext, EveMiddleware, App> {
	let mut app = create_eve_app(&app);

	// List all deployments
	app.get(
		"/",
		&[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::deployment::LIST,
				closure_as_pinned_box!(|mut context| {
					let org_id_string = context
						.get_param(request_keys::ORGANISATION_ID)
						.unwrap();
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
			EveMiddleware::CustomFunction(pin_fn!(list_deployments)),
		],
	);

	// Create a new deployment
	app.post(
		"/",
		&[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::deployment::CREATE,
				closure_as_pinned_box!(|mut context| {
					let org_id_string = context
						.get_param(request_keys::ORGANISATION_ID)
						.unwrap();
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
			EveMiddleware::CustomFunction(pin_fn!(create_deployment)),
		],
	);

	// Get info about a deployment
	app.get(
		"/:deploymentId/",
		&[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::deployment::INFO,
				closure_as_pinned_box!(|mut context| {
					let deployment_id_string =
						context.get_param(request_keys::DEPLOYMENT_ID).unwrap();
					let deployment_id = hex::decode(&deployment_id_string);
					if deployment_id.is_err() {
						context.status(400).json(error!(WRONG_PARAMETERS));
						return Ok((context, None));
					}
					let deployment_id = deployment_id.unwrap();

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
		&[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::deployment::DELETE,
				closure_as_pinned_box!(|mut context| {
					let deployment_id_string =
						context.get_param(request_keys::DEPLOYMENT_ID).unwrap();
					let deployment_id = hex::decode(&deployment_id_string);
					if deployment_id.is_err() {
						context.status(400).json(error!(WRONG_PARAMETERS));
						return Ok((context, None));
					}
					let deployment_id = deployment_id.unwrap();

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
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
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
			request_keys::DEPLOYMENT_ID: hex::encode(deployment.id),
			request_keys::NAME: deployment.name,
			request_keys::REGISTRY: deployment.registry,
			request_keys::IMAGE_NAME: deployment.image_name,
			request_keys::IMAGE_TAG: deployment.image_tag,
			request_keys::DOMAIN_ID: hex::encode(deployment.domain_id),
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
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let organisation_id =
		hex::decode(context.get_param(request_keys::ORGANISATION_ID).unwrap())
			.unwrap();
	let body = context.get_body_object().clone();

	let name = if let Some(Value::String(val)) = body.get(request_keys::NAME) {
		val
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let registry =
		if let Some(Value::String(val)) = body.get(request_keys::REGISTRY) {
			val
		} else {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		};

	let image_name =
		if let Some(Value::String(val)) = body.get(request_keys::IMAGE_NAME) {
			val
		} else {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		};

	let image_tag =
		if let Some(Value::String(val)) = body.get(request_keys::IMAGE_TAG) {
			val
		} else {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		};

	let domain_id =
		if let Some(Value::String(val)) = body.get(request_keys::DOMAIN_ID) {
			val
		} else {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		};

	let sub_domain =
		if let Some(Value::String(val)) = body.get(request_keys::SUB_DOMAIN) {
			val
		} else {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		};

	let path = if let Some(Value::String(val)) = body.get(request_keys::PATH) {
		val
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let port = if let Some(Value::Number(val)) = body.get(request_keys::PORT) {
		if let Some(val) = val.as_u64() {
			val as u16
		} else if let Some(val) = val.as_i64() {
			val as u16
		} else if let Some(val) = val.as_f64() {
			val as u16
		} else {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		}
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	match registry.as_str() {
		"registry.docker.vicara.co" | "registry.hub.docker.com" => (),
		_ => {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		}
	}

	let domain_id = if let Ok(domain_id) = hex::decode(domain_id) {
		domain_id
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let deployment = db::get_deployment_by_entry_point(
		context.get_mysql_connection(),
		&domain_id,
		sub_domain,
		path,
	)
	.await?;
	if deployment.is_some() {
		context.status(404).json(error!(RESOURCE_EXISTS));
		return Ok(context);
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
		request_keys::DEPLOYMENT_ID: hex::encode(deployment_id)
	}));
	Ok(context)
}

async fn get_deployment_info(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let deployment_id =
		hex::decode(context.get_param(request_keys::DEPLOYMENT_ID).unwrap())
			.unwrap();
	let deployment = db::get_deployment_by_id(
		context.get_mysql_connection(),
		&deployment_id,
	)
	.await?;
	if deployment.is_none() {
		context.status(404).json(error!(RESOURCE_DOES_NOT_EXIST));
		return Ok(context);
	}
	let deployment = deployment.unwrap();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::DEPLOYMENT: {
			request_keys::DEPLOYMENT_ID: hex::encode(deployment.id),
			request_keys::NAME: deployment.name,
			request_keys::REGISTRY: deployment.registry,
			request_keys::IMAGE_NAME: deployment.image_name,
			request_keys::IMAGE_TAG: deployment.image_tag,
			request_keys::DOMAIN_ID: hex::encode(deployment.domain_id),
			request_keys::SUB_DOMAIN: deployment.sub_domain,
			request_keys::PATH: deployment.path,
			request_keys::PORT: deployment.port,
		}
	}));
	Ok(context)
}

async fn delete_deployment(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let deployment_id =
		hex::decode(context.get_param(request_keys::DEPLOYMENT_ID).unwrap())
			.unwrap();
	let deployment = db::get_deployment_by_id(
		context.get_mysql_connection(),
		&deployment_id,
	)
	.await?;
	if deployment.is_none() {
		context.status(404).json(error!(RESOURCE_DOES_NOT_EXIST));
		return Ok(context);
	}

	db::delete_deployment_by_id(context.get_mysql_connection(), &deployment_id)
		.await?;

	// TODO stop and delete the container running the image, if it exists

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}
