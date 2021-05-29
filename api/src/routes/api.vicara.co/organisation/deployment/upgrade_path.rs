use api_macros::closure_as_pinned_box;
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use serde_json::json;

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::{db_mapping::MachineType, rbac::permissions},
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

	// List all upgrade paths
	app.get(
		"/upgrade-path/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::deployment::upgrade_path::LIST,
				closure_as_pinned_box!(|mut context| {
					let organisation_id_string = context
						.get_param(request_keys::ORGANISATION_ID)
						.unwrap();
					let organisation_id = hex::decode(&organisation_id_string)
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
			EveMiddleware::CustomFunction(pin_fn!(list_upgrade_paths)),
		],
	);

	// Create a new upgrade path
	app.post(
		"/upgrade-path/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::deployment::upgrade_path::CREATE,
				closure_as_pinned_box!(|mut context| {
					let organisation_id_string = context
						.get_param(request_keys::ORGANISATION_ID)
						.unwrap();
					let organisation_id = hex::decode(&organisation_id_string)
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
			EveMiddleware::CustomFunction(pin_fn!(create_upgrade_path)),
		],
	);

	// Get info of an upgrade path
	app.get(
		"/upgrade-path/:upgradePathId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::deployment::upgrade_path::INFO,
				closure_as_pinned_box!(|mut context| {
					let upgrade_path_id_string = context
						.get_param(request_keys::UPGRADE_PATH_ID)
						.unwrap();
					let upgrade_path_id = hex::decode(&upgrade_path_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&upgrade_path_id,
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
			EveMiddleware::CustomFunction(pin_fn!(get_upgrade_path_details)),
		],
	);

	// Modify an upgrade path
	app.post(
		"/upgrade-path/:upgradePathId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::deployment::upgrade_path::EDIT,
				closure_as_pinned_box!(|mut context| {
					let upgrade_path_id_string = context
						.get_param(request_keys::UPGRADE_PATH_ID)
						.unwrap();
					let upgrade_path_id = hex::decode(&upgrade_path_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&upgrade_path_id,
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
			EveMiddleware::CustomFunction(pin_fn!(update_upgrade_path)),
		],
	);

	// Delete an upgrade path
	app.delete(
		"/upgrade-path/:upgradePathId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::deployment::upgrade_path::DELETE,
				closure_as_pinned_box!(|mut context| {
					let upgrade_path_id_string = context
						.get_param(request_keys::UPGRADE_PATH_ID)
						.unwrap();
					let upgrade_path_id = hex::decode(&upgrade_path_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&upgrade_path_id,
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
			EveMiddleware::CustomFunction(pin_fn!(delete_upgrade_path)),
		],
	);

	app
}

async fn list_upgrade_paths(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let organisation_id =
		hex::decode(context.get_param(request_keys::ORGANISATION_ID).unwrap())
			.unwrap();
	let upgrade_paths = db::get_deployment_upgrade_paths_in_organisation(
		context.get_database_connection(),
		&organisation_id,
	)
	.await?
	.into_iter()
	.map(|upgrade_path| {
		json!({
			request_keys::ID: hex::encode(upgrade_path.id),
			request_keys::NAME: upgrade_path.name
		})
	})
	.collect::<Vec<_>>();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::UPGRADE_PATHS: upgrade_paths
	}));
	Ok(context)
}

async fn create_upgrade_path(
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
	let machine_types_values = body
		.get(request_keys::MACHINE_TYPES)
		.map(|value| value.as_array())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let mut machine_types: Vec<MachineType> = vec![];
	for machine_type_value in machine_types_values {
		let machine_type: MachineType =
			serde_json::from_value(machine_type_value.clone())
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())?;
		machine_types.push(machine_type);
	}

	let upgrade_path_id =
		service::create_deployment_upgrade_path_in_organisation(
			context.get_database_connection(),
			&organisation_id,
			name,
			&machine_types,
		)
		.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::UPGRADE_PATH_ID: hex::encode(upgrade_path_id.as_bytes())
	}));
	Ok(context)
}

async fn get_upgrade_path_details(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let upgrade_path_id =
		hex::decode(context.get_param(request_keys::UPGRADE_PATH_ID).unwrap())
			.unwrap();

	let upgrade_path = db::get_deployment_upgrade_path_by_id(
		context.get_database_connection(),
		&upgrade_path_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let machine_types = db::get_machine_types_in_deployment_upgrade_path(
		context.get_database_connection(),
		&upgrade_path_id,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ID: hex::encode(upgrade_path.id),
		request_keys::NAME: upgrade_path.name,
		request_keys::MACHINE_TYPES: machine_types
	}));
	Ok(context)
}

async fn delete_upgrade_path(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let upgrade_path_id =
		hex::decode(context.get_param(request_keys::UPGRADE_PATH_ID).unwrap())
			.unwrap();

	db::get_deployment_upgrade_path_by_id(
		context.get_database_connection(),
		&upgrade_path_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	db::remove_all_machine_types_for_deployment_upgrade_path(
		context.get_database_connection(),
		&upgrade_path_id,
	)
	.await?;
	db::delete_deployment_upgrade_path_by_id(
		context.get_database_connection(),
		&upgrade_path_id,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

async fn update_upgrade_path(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let upgrade_path_id =
		hex::decode(context.get_param(request_keys::UPGRADE_PATH_ID).unwrap())
			.unwrap();
	let body = context.get_body_object().clone();
	let name = body
		.get(request_keys::NAME)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let machine_types_values = body
		.get(request_keys::MACHINE_TYPES)
		.map(|value| value.as_array())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let mut machine_types: Vec<MachineType> = vec![];
	for machine_type_value in machine_types_values {
		let machine_type: MachineType =
			serde_json::from_value(machine_type_value.clone())
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())?;
		machine_types.push(machine_type);
	}

	service::update_deployment_upgrade_path(
		context.get_database_connection(),
		&upgrade_path_id,
		name,
		&machine_types,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}
