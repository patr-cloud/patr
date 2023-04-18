use api_macros::closure_as_pinned_box;
use api_models::{
	models::workspace::infrastructure::database::{
		Connection,
		CreateDatabaseRequest,
		CreateDatabaseResponse,
		Database,
		DeleteDatabaseResponse,
		GetDatabaseInfoResponse,
		ListDatabasesResponse,
	},
	utils::Uuid,
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};

use crate::{
	app::{create_eve_app, App},
	db,
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
				permission:
					permissions::workspace::infrastructure::patr_database::LIST,
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
				permission: permissions::workspace::infrastructure::patr_database::CREATE,
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
				permission:
					permissions::workspace::infrastructure::patr_database::INFO,
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
			EveMiddleware::CustomFunction(pin_fn!(get_patr_database_info)),
		],
	);

	app.delete(
		"/:databaseId/",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission: permissions::workspace::infrastructure::patr_database::DELETE,
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
			EveMiddleware::CustomFunction(pin_fn!(delete_patr_database)),
		],
	);
	app.post(
		"/:databaseId/",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::patr_database::EDIT,
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
			EveMiddleware::CustomFunction(pin_fn!(modify_database_cluster)),
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
	let database_clusters = db::get_all_patr_database_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.into_iter()
	.map(|database| Database {
		id: database.id,
		name: database.name,
		database_name: database.db_name,
		engine: database.engine,
		version: database.version,
		database_plan: database.database_plan,
		region: database.region,
		status: database.status,
		connection: Connection {
			host: database.host,
			port: database.port,
			username: database.username,
			password: database.password,
		},
		replica_numbers: database.replica_numbers,
	})
	.collect::<Vec<_>>();

	log::trace!(
		"request_id: {} - Returning all database cluster info",
		request_id
	);

	context.success(ListDatabasesResponse {
		databases: database_clusters,
	});

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
	let config = context.get_state().config.clone();

	log::trace!("request_id: {} - Creating database cluster", request_id);
	let CreateDatabaseRequest {
		// use workspace_id from query param as this value will be default
		workspace_id: _,
		name,
		database_name,
		engine,
		database_plan,
		region,
		replica_numbers,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let database_id = service::create_patr_database_in_workspace(
		context.get_database_connection(),
		&name,
		&database_name,
		&engine,
		&database_plan,
		&region,
		&workspace_id,
		&config,
		&request_id,
		replica_numbers,
	)
	.await?;

	context.commit_database_transaction().await?;

	service::queue_check_and_update_database_status(
		&workspace_id,
		&database_id,
		&config,
		&request_id,
	)
	.await?;

	let _ = service::get_internal_metrics(
		context.get_database_connection(),
		"A patr database instance has been created",
	)
	.await;

	context.success(CreateDatabaseResponse { id: database_id });
	Ok(context)
}

async fn get_patr_database_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	let database_id =
		Uuid::parse_str(context.get_param(request_keys::DATABASE_ID).unwrap())
			.unwrap();

	log::trace!("request_id: {} - Getting database info", request_id);
	let database = db::get_patr_database_by_id(
		context.get_database_connection(),
		&database_id,
	)
	.await?
	.map(|database| Database {
		id: database.id,
		name: database.name,
		database_name: database.db_name,
		engine: database.engine,
		version: database.version,
		database_plan: database.database_plan,
		region: database.region,
		status: database.status,
		connection: Connection {
			host: database.host,
			port: database.port,
			username: database.username,
			password: database.password,
		},
		replica_numbers: database.replica_numbers,
	})
	.status(400)
	.body(error!(WRONG_PARAMETERS).to_string())?;
	log::trace!("request_id: {} - Returning database info", request_id);

	context.success(GetDatabaseInfoResponse { database });

	Ok(context)
}

async fn delete_patr_database(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	let user_id = context.get_token_data().unwrap().user_id().clone();

	let database_id =
		Uuid::parse_str(context.get_param(request_keys::DATABASE_ID).unwrap())
			.unwrap();

	let database = db::get_patr_database_by_id(
		context.get_database_connection(),
		&database_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let config = context.get_state().config.clone();

	log::trace!("request_id: {} - Deleting database cluster", request_id);
	service::delete_patr_database(
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
		"A patr database instance has been deleted",
	)
	.await;

	context.success(DeleteDatabaseResponse {});
	Ok(context)
}

async fn modify_database_cluster(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	let database_id =
		Uuid::parse_str(context.get_param(request_keys::DATABASE_ID).unwrap())
			.unwrap();

	let replica_numbers = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let config = context.get_state().config.clone();
	log::trace!("request_id: {} - Modifying database cluster", request_id);
	service::modify_patr_database(
		context.get_database_connection(),
		&database_id,
		&config,
		&request_id,
		replica_numbers,
	)
	.await?;

	Ok(context)
}
