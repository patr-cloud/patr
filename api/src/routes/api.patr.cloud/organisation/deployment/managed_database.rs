use api_macros::closure_as_pinned_box;
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use reqwest::Client;
use serde_json::json;

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::{db_mapping::DatabasePlan, rbac::permissions},
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
	app.get(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::managed_database::LIST,
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
			EveMiddleware::CustomFunction(pin_fn!(list_all_database_clusters)),
		],
	);

	app.post(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::managed_database::CREATE,
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
			EveMiddleware::CustomFunction(pin_fn!(create_database_cluster)),
		],
	);

	app.get(
		"/:resourceId/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::managed_database::INFO,
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
			EveMiddleware::CustomFunction(pin_fn!(get_managed_database_info)),
		],
	);

	app.delete(
		"/:resourceId/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::managed_database::DELETE,
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
			EveMiddleware::CustomFunction(pin_fn!(delete_managed_database)),
		],
	);
	app
}

async fn list_all_database_clusters(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let organisation_id =
		hex::decode(context.get_param(request_keys::ORGANISATION_ID).unwrap())
			.unwrap();

	let config = context.get_state().config.clone();
	let database_clusters =
		service::get_all_database_clusters_for_organisation(
			context.get_database_connection(),
			config,
			&organisation_id,
		)
		.await?
		.into_iter()
		.map(|response| {
			json!({
				request_keys::ID: response.database.id,
				request_keys::NAME: response.database.name,
				request_keys::ENGINE: response.database.engine,
				request_keys::VERSION: response.database.version,
				request_keys::NUM_NODES: response.database.num_nodes,
				request_keys::CREATED_AT: response.database.created_at,
				request_keys::CONNECTION: {
					request_keys::HOST: response.database.connection.host,
					request_keys::USERNAME: response.database.connection.user,
					request_keys::PASSWORD: response.database.connection.password,
					request_keys::PORT: response.database.connection.port
				}
			})
		})
		.collect::<Vec<_>>();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::DATABASE_CLUSTERS: database_clusters
	}));

	Ok(context)
}

async fn create_database_cluster(
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

	let db_name = body
		.get(request_keys::DATABASE_NAME)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let version = body
		.get(request_keys::VERSION)
		.map(|value| {
			value
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	let engine = body
		.get(request_keys::ENGINE)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	// only compulsory for digital ocean
	let num_nodes = body
		.get(request_keys::NUM_NODES)
		.map(|value| {
			value
				.as_u64()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	let region = body
		.get(request_keys::REGION)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let database_plan = body
		.get(request_keys::DATABASE_PLAN)
		.map(|value| value.as_str())
		.flatten()
		.map(|c| c.parse::<DatabasePlan>().ok())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let config = context.get_state().config.clone();

	service::create_database_cluster(
		config,
		name,
		db_name,
		version,
		engine,
		num_nodes,
		region,
		&organisation_id,
		database_plan,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::STATUS: "creating"
	}));
	Ok(context)
}

async fn get_managed_database_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let resource_id =
		hex::decode(context.get_param(request_keys::RESOURCE_ID).unwrap())
			.unwrap();

	let config = context.get_state().config.clone();
	let (database_info, status) =
		service::get_managed_database_info_for_organisation(
			context.get_database_connection(),
			&config,
			&resource_id,
		)
		.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::DATABASE_CLUSTER: {
			request_keys::ID: hex::encode(resource_id),
			request_keys::NAME: database_info.name,
			request_keys::ENGINE: database_info.engine,
			request_keys::VERSION: database_info.version,
			request_keys::NUM_NODES: database_info.num_nodes,
			request_keys::CREATED_AT: database_info.created_at,
			request_keys::CONNECTION: {
				request_keys::HOST: database_info.connection.host,
				request_keys::USERNAME: database_info.connection.user,
				request_keys::PASSWORD: database_info.connection.password,
				request_keys::PORT: database_info.connection.port
			}
		},
		request_keys::STATUS: status
	}));

	Ok(context)
}

async fn delete_managed_database(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let resource_id =
		hex::decode(context.get_param(request_keys::RESOURCE_ID).unwrap())
			.unwrap();
	let client = Client::new();
	let config = context.get_state().config.clone();

	service::delete_managed_database(
		context.get_database_connection(),
		&config,
		&resource_id,
		&client,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::STATUS: "deleted"
	}));

	Ok(context)
}
