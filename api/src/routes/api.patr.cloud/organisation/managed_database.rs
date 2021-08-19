use api_macros::closure_as_pinned_box;
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use serde_json::json;

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
			EveMiddleware::CustomFunction(pin_fn!(create_new_database_cluster)),
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
				request_keys::HOST: response.database.connection.host,
				request_keys::USERNAME: response.database.connection.user,
				request_keys::PASSWORD: response.database.connection.password,
				request_keys::PORT: response.database.connection.port
			})
		})
		.collect::<Vec<_>>();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::DATABASE_CLUSTERS: database_clusters
	}));

	Ok(context)
}

async fn create_new_database_cluster(
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

	let num_nodes = body
		.get(request_keys::NUM_NODES)
		.map(|value| value.as_u64())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let region = body
		.get(request_keys::REGION)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let config = context.get_state().config.clone();

	let database_config = service::create_new_database_cluster(
		context.get_database_connection(),
		config,
		name,
		version,
		engine,
		num_nodes,
		region,
		&organisation_id,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::NAME: database_config.name,
		request_keys::ENGINE: database_config.engine,
		request_keys::VERSION: database_config.version,
		request_keys::URI: database_config.connection.uri,
		request_keys::HOST: database_config.connection.host,
		request_keys::PORT: database_config.connection.port,
		request_keys::USERNAME: database_config.connection.user,
		request_keys::PASSWORD: database_config.connection.password
	}));
	Ok(context)
}
