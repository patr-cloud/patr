use std::convert::TryInto;

use api_macros::closure_as_pinned_box;
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use serde_json::{json, Map, Value};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::{
		db_mapping::{DeploymentEntryPoint, DeploymentEntryPointValue},
		rbac::permissions,
	},
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

pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut app = create_eve_app(app);

	// List all entry points
	app.get(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::deployment::entry_point::LIST,
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
			EveMiddleware::CustomFunction(pin_fn!(list_entry_points)),
		],
	);

	// Create a new entry point
	app.post(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::deployment::entry_point::CREATE,
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
			EveMiddleware::CustomFunction(pin_fn!(create_entry_point)),
		],
	);

	// Edit an entry point
	app.post(
		"/:entryPointId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::deployment::entry_point::EDIT,
				closure_as_pinned_box!(|mut context| {
					let entry_point_id_string = context
						.get_param(request_keys::ENTRY_POINT_ID)
						.unwrap();
					let entry_point_id = hex::decode(&entry_point_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&entry_point_id,
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
			EveMiddleware::CustomFunction(pin_fn!(edit_entry_point)),
		],
	);

	// Delete an entry point
	app.delete(
		"/:entryPointId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::deployment::entry_point::DELETE,
				closure_as_pinned_box!(|mut context| {
					let entry_point_id_string = context
						.get_param(request_keys::ENTRY_POINT_ID)
						.unwrap();
					let entry_point_id = hex::decode(&entry_point_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&entry_point_id,
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
			EveMiddleware::CustomFunction(pin_fn!(delete_entry_point)),
		],
	);

	app
}

async fn list_entry_points(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let organisation_id =
		hex::decode(context.get_param(request_keys::ORGANISATION_ID).unwrap())
			.unwrap();
	let query = context.get_request().get_query().clone();
	let domain_id = query
		.get(request_keys::DOMAIN_ID)
		.map(|value| {
			hex::decode(value)
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;
	let domain_id = domain_id.as_deref();

	let deployment_id = query
		.get(request_keys::DEPLOYMENT_ID)
		.map(|value| {
			hex::decode(value)
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;
	let deployment_id = deployment_id.as_deref();

	let mut entry_points = db::get_deployment_entry_points_in_organisation(
		context.get_database_connection(),
		&organisation_id,
	)
	.await?;

	if let Some(domain_id) = domain_id {
		entry_points = entry_points
			.into_iter()
			.filter(|entry_point| entry_point.domain_id == domain_id)
			.collect();
	}

	if let Some(expected_deployment_id) = deployment_id {
		entry_points = entry_points
			.into_iter()
			.filter(|entry_point| {
				if let DeploymentEntryPointValue::Deployment {
					deployment_id,
					deployment_port: _,
				} = &entry_point.entry_point_type
				{
					deployment_id == expected_deployment_id
				} else {
					false
				}
			})
			.collect();
	}

	let entry_points = entry_points
		.into_iter()
		.map(|entry_point| {
			let DeploymentEntryPoint {
				id,
				sub_domain,
				domain_id,
				path,
				entry_point_type,
			} = entry_point;
			let mut map = Map::new();
			map.insert(
				request_keys::ID.to_string(),
				Value::String(hex::encode(id)),
			);
			if let Some(sub_domain) = sub_domain {
				map.insert(
					request_keys::SUB_DOMAIN.to_string(),
					Value::String(sub_domain),
				);
			}
			map.insert(
				request_keys::DOMAIN_ID.to_string(),
				Value::String(hex::encode(domain_id)),
			);
			map.insert(request_keys::PATH.to_string(), Value::String(path));
			match entry_point_type {
				DeploymentEntryPointValue::Deployment {
					deployment_id,
					deployment_port,
				} => {
					map.insert(
						request_keys::ENTRY_POINT_TYPE.to_string(),
						Value::String("deployment".to_string()),
					);
					map.insert(
						request_keys::DEPLOYMENT_ID.to_string(),
						Value::String(hex::encode(deployment_id)),
					);
					map.insert(
						request_keys::PORT.to_string(),
						Value::Number(deployment_port.into()),
					);
				}
				DeploymentEntryPointValue::Redirect { url } => {
					map.insert(
						request_keys::ENTRY_POINT_TYPE.to_string(),
						Value::String("redirect".to_string()),
					);
					map.insert(
						request_keys::URL.to_string(),
						Value::String(url),
					);
				}
				DeploymentEntryPointValue::Proxy { url } => {
					map.insert(
						request_keys::ENTRY_POINT_TYPE.to_string(),
						Value::String("proxy".to_string()),
					);
					map.insert(
						request_keys::URL.to_string(),
						Value::String(url),
					);
				}
			}
		})
		.collect::<Vec<_>>();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ENTRY_POINTS: entry_points
	}));
	Ok(context)
}

async fn create_entry_point(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let organisation_id =
		hex::decode(context.get_param(request_keys::ORGANISATION_ID).unwrap())
			.unwrap();
	let body = context.get_body_object().clone();
	let sub_domain = body
		.get(request_keys::SUB_DOMAIN)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let domain_id = body
		.get(request_keys::DOMAIN_ID)
		.map(|value| value.as_str())
		.flatten()
		.map(|value| hex::decode(value).ok())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let path = body
		.get(request_keys::PATH)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let entry_point_type = body
		.get(request_keys::ENTRY_POINT_TYPE)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let deployment_id = body
		.get(request_keys::DEPLOYMENT_ID)
		.map(|value| {
			value
				.as_str()
				.map(|value| hex::decode(value).ok())
				.flatten()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;
	let deployment_port = body
		.get(request_keys::DEPLOYMENT_ID)
		.map(|value| {
			let number = match value {
				Value::Number(number) => {
					if let Some(number) = number.as_u64() {
						number.try_into().ok()
					} else if let Some(number) = number.as_i64() {
						number.try_into().ok()
					} else {
						None
					}
				}
				Value::String(number) => number.parse::<u16>().ok(),
				_ => None,
			};
			number
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;
	let url = body
		.get(request_keys::DEPLOYMENT_ID)
		.map(|value| {
			value
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	let entry_point_uuid =
		service::create_deployment_entry_point_in_organisation(
			context.get_database_connection(),
			&organisation_id,
			sub_domain,
			&domain_id,
			path,
			entry_point_type,
			deployment_id.as_deref(),
			deployment_port,
			url,
		)
		.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ENTRY_POINT_ID: hex::encode(entry_point_uuid.as_bytes())
	}));
	Ok(context)
}

async fn edit_entry_point(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();
	let entry_point_id =
		hex::decode(context.get_param(request_keys::ENTRY_POINT_ID).unwrap())
			.unwrap();
	let entry_point_type = body
		.get(request_keys::ENTRY_POINT_TYPE)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let deployment_id = body
		.get(request_keys::DEPLOYMENT_ID)
		.map(|value| {
			value
				.as_str()
				.map(|value| hex::decode(value).ok())
				.flatten()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;
	let deployment_port = body
		.get(request_keys::DEPLOYMENT_ID)
		.map(|value| {
			let number = match value {
				Value::Number(number) => {
					if let Some(number) = number.as_u64() {
						number.try_into().ok()
					} else if let Some(number) = number.as_i64() {
						number.try_into().ok()
					} else {
						None
					}
				}
				Value::String(number) => number.parse::<u16>().ok(),
				_ => None,
			};
			number
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;
	let url = body
		.get(request_keys::DEPLOYMENT_ID)
		.map(|value| {
			value
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	service::update_deployment_entry_point(
		context.get_database_connection(),
		&entry_point_id,
		entry_point_type,
		deployment_id.as_deref(),
		deployment_port,
		url,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

async fn delete_entry_point(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let entry_point_id =
		hex::decode(context.get_param(request_keys::ENTRY_POINT_ID).unwrap())
			.unwrap();

	db::delete_deployment_entry_point_by_id(
		context.get_database_connection(),
		&entry_point_id,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}
