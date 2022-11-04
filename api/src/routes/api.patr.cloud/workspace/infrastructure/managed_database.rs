use api_macros::closure_as_pinned_box;
use api_models::utils::Uuid;
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use serde_json::json;

use crate::{
	app::{create_eve_app, App},
	db::{self, ManagedDatabasePlan},
	error,
	models::{rbac::permissions, ResourceType},
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
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission: permissions::workspace::infrastructure::managed_database::LIST,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(list_all_database_clusters)),
		],
	);

	app.post(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission: permissions::workspace::infrastructure::managed_database::CREATE,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(create_database_cluster)),
		],
	);

	app.get(
		"/:databaseId/",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission: permissions::workspace::infrastructure::managed_database::INFO,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let database_id =
						context.get_param(request_keys::DATABASE_ID).unwrap();
					let database_id_string = Uuid::parse_str(database_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&database_id_string,
					)
					.await?
					.filter(|value| value.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(get_managed_database_info)),
		],
	);

	app.delete(
		"/:databaseId/",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission: permissions::workspace::infrastructure::managed_database::DELETE,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let database_id =
						context.get_param(request_keys::DATABASE_ID).unwrap();
					let database_id_string = Uuid::parse_str(database_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&database_id_string,
					)
					.await?
					.filter(|value| value.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(delete_managed_database)),
		],
	);
	app
}

async fn list_all_database_clusters(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	log::trace!(
		"request_id: {} - Getting all database cluster info from db",
		request_id
	);
	let database_clusters = db::get_all_database_clusters_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.into_iter()
	.map(|database| {
		json!({
			request_keys::ID: database.id,
			request_keys::NAME: database.name,
			request_keys::DATABASE_NAME: database.db_name,
			request_keys::ENGINE: database.engine.to_string(),
			request_keys::VERSION: database.version,
			request_keys::NUM_NODES: database.num_nodes,
			request_keys::DATABASE_PLAN: database.database_plan.to_string(),
			request_keys::REGION: database.region,
			request_keys::STATUS: database.status.to_string(),
			request_keys::PUBLIC_CONNECTION: {
				request_keys::HOST: database.host,
				request_keys::PORT: database.port,
				request_keys::USERNAME: database.username,
				request_keys::PASSWORD: database.password,
			}
		})
	})
	.collect::<Vec<_>>();

	log::trace!(
		"request_id: {} - Returning all database cluster info",
		request_id
	);

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::DATABASES: database_clusters
	}));

	Ok(context)
}

async fn create_database_cluster(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let body = context.get_body_object().clone();
	let config = context.get_state().config.clone();

	log::trace!("request_id: {} - Creating database cluster", request_id);
	let name = body
		.get(request_keys::NAME)
		.and_then(|value| value.as_str())
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?
		.trim();

	let db_name = body
		.get(request_keys::DATABASE_NAME)
		.and_then(|value| value.as_str())
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?
		.trim();

	let engine = body
		.get(request_keys::ENGINE)
		.and_then(|value| value.as_str())
		.and_then(|engine| engine.parse().ok())
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

	let database_plan = body
		.get(request_keys::DATABASE_PLAN)
		.and_then(|value| value.as_str())
		.and_then(|c| c.parse::<ManagedDatabasePlan>().ok())
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let region = body
		.get(request_keys::REGION)
		.and_then(|value| value.as_str())
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let database_id = service::create_managed_database_in_workspace(
		context.get_database_connection(),
		name,
		db_name,
		&engine,
		version,
		num_nodes,
		&database_plan,
		region,
		&workspace_id,
		&config,
		&request_id,
	)
	.await?;

	let _ = service::get_internal_metrics(
		context.get_database_connection(),
		"A database instance has been created",
	)
	.await;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::DATABASE_ID: database_id.as_str()
	}));
	Ok(context)
}

async fn get_managed_database_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	let database_id =
		Uuid::parse_str(context.get_param(request_keys::DATABASE_ID).unwrap())
			.unwrap();

	log::trace!("request_id: {} - Getting database info", request_id);
	let database = db::get_managed_database_by_id(
		context.get_database_connection(),
		&database_id,
	)
	.await?
	.status(400)
	.body(error!(WRONG_PARAMETERS).to_string())?;
	log::trace!("request_id: {} - Returning database info", request_id);

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::DATABASE_ID: database.id,
		request_keys::NAME: database.name,
		request_keys::DATABASE_NAME: database.db_name,
		request_keys::ENGINE: database.engine.to_string(),
		request_keys::VERSION: database.version,
		request_keys::NUM_NODES: database.num_nodes,
		request_keys::DATABASE_PLAN: database.database_plan.to_string(),
		request_keys::REGION: database.region,
		request_keys::STATUS: database.status.to_string(),
		request_keys::PUBLIC_CONNECTION: {
			request_keys::HOST: database.host,
			request_keys::PORT: database.port,
			request_keys::USERNAME: database.username,
			request_keys::PASSWORD: database.password,
		}
	}));

	Ok(context)
}

async fn delete_managed_database(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	let user_id = context.get_token_data().unwrap().user_id().clone();

	let database_id =
		Uuid::parse_str(context.get_param(request_keys::DATABASE_ID).unwrap())
			.unwrap();

	let database = db::get_managed_database_by_id(
		context.get_database_connection(),
		&database_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let config = context.get_state().config.clone();

	log::trace!("request_id: {} - Deleting database cluster", request_id);
	service::delete_managed_database(
		context.get_database_connection(),
		&database_id,
		&config,
		&request_id,
	)
	.await?;

	// Commiting transaction so that even if the mailing function fails the
	// resource should be deleted
	context.commit_database_transaction().await?;

	service::resource_delete_action_email(
		context.get_database_connection(),
		&database.name,
		&database.workspace_id,
		&ResourceType::ManagedDatabase,
		&user_id,
	)
	.await?;

	let _ = service::get_internal_metrics(
		context.get_database_connection(),
		"A database instance has been deleted",
	)
	.await;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}
