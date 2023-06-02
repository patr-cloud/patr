use api_macros::closure_as_pinned_box;
use api_models::{
	models::workspace::infrastructure::database::{
		ChangeDatabasePasswordRequest,
		ChangeDatabasePasswordResponse,
		Connection,
		CreateDatabaseRequest,
		CreateDatabaseResponse,
		Database,
		DeleteDatabaseResponse,
		GetDatabaseInfoResponse,
		ListDatabasesResponse,
		ManagedDatabaseEngine,
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
	utils::{constants::request_keys, Error, EveContext, EveMiddleware},
};

pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, Error> {
	let mut app = create_eve_app(app);
	app.get(
		"/",
		[
			EveMiddleware::WorkspaceMemberAuthenticator {
				is_api_token_allowed: true,
				requested_workspace: closure_as_pinned_box!(|context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					Ok((context, workspace_id))
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
							.status(404)?
							.json(error!(RESOURCE_DOES_NOT_EXIST)).await?;
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
					permissions::workspace::infrastructure::managed_database::INFO,
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
							.status(404)?
							.json(error!(RESOURCE_DOES_NOT_EXIST)).await?;
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
							.status(404)?
							.json(error!(RESOURCE_DOES_NOT_EXIST)).await?;
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(delete_managed_database)),
		],
	);

	app.post(
		"/:databaseId/change-password",
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
			EveMiddleware::CustomFunction(pin_fn!(change_database_password)),
		],
	);

	app
}

async fn list_all_database_clusters(
	mut context: EveContext,
	_: NextHandler<EveContext, Error>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let user_token = context.get_token_data().status(500)?.clone();

	log::trace!(
		"request_id: {} - Getting all database cluster info from db",
		request_id
	);
	let database_clusters = db::get_all_managed_database_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.into_iter()
	.filter(|db| {
		user_token.has_access_for_requested_action(
			&workspace_id,
			&db.id,
			permissions::workspace::infrastructure::managed_database::INFO,
		)
	})
	.map(|database| Database {
		id: database.id.to_owned(),
		name: database.name,
		engine: database.engine.to_owned(),
		version: match database.engine {
			ManagedDatabaseEngine::Postgres => "12".to_string(),
			ManagedDatabaseEngine::Mysql => "8".to_string(),
			ManagedDatabaseEngine::Mongo => "4".to_string(),
			ManagedDatabaseEngine::Redis => "6".to_string(),
		},
		database_plan_id: database.database_plan_id,
		region: database.region,
		status: database.status,
		connection: Connection {
			host: format!("db-{0}.svc.local", database.id),
			port: match database.engine {
				ManagedDatabaseEngine::Postgres => 5432,
				ManagedDatabaseEngine::Mysql => 3306,
				ManagedDatabaseEngine::Mongo => 27017,
				ManagedDatabaseEngine::Redis => 6379,
			},
			username: database.username,
		},
	})
	.collect::<Vec<_>>();

	log::trace!(
		"request_id: {} - Returning all database cluster info",
		request_id
	);

	context
		.success(ListDatabasesResponse {
			databases: database_clusters,
		})
		.await?;
	Ok(context)
}

async fn create_database_cluster(
	mut context: EveContext,
	_: NextHandler<EveContext, Error>,
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
		engine,
		database_plan_id,
		region,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let (database_id, password) =
		service::create_managed_database_in_workspace(
			context.get_database_connection(),
			&name,
			&engine,
			&database_plan_id,
			&region,
			&workspace_id,
			&request_id,
		)
		.await?;

	context.commit_database_transaction().await?;

	service::queue_check_and_update_database_status(
		&workspace_id,
		&database_id,
		&config,
		&request_id,
		&password,
	)
	.await?;

	let _ = service::get_internal_metrics(
		context.get_database_connection(),
		"A patr database instance has been created",
	)
	.await;

	context
		.success(CreateDatabaseResponse {
			id: database_id,
			password,
		})
		.await?;
	Ok(context)
}

async fn get_managed_database_info(
	mut context: EveContext,
	_: NextHandler<EveContext, Error>,
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
	.map(|database| Database {
		id: database.id.to_owned(),
		name: database.name,
		engine: database.engine.to_owned(),
		version: match database.engine {
			ManagedDatabaseEngine::Postgres => "12".to_string(),
			ManagedDatabaseEngine::Mysql => "8".to_string(),
			ManagedDatabaseEngine::Mongo => "4".to_string(),
			ManagedDatabaseEngine::Redis => "6".to_string(),
		},
		database_plan_id: database.database_plan_id,
		region: database.region,
		status: database.status,
		connection: Connection {
			host: format!("db-{0}.svc.local", database.id),
			port: match database.engine {
				ManagedDatabaseEngine::Postgres => 5432,
				ManagedDatabaseEngine::Mysql => 3306,
				ManagedDatabaseEngine::Mongo => 27017,
				ManagedDatabaseEngine::Redis => 6379,
			},
			username: database.username,
		},
	})
	.status(400)
	.body(error!(WRONG_PARAMETERS).to_string())?;
	log::trace!("request_id: {} - Returning database info", request_id);

	context
		.success(GetDatabaseInfoResponse { database })
		.await?;

	Ok(context)
}

async fn delete_managed_database(
	mut context: EveContext,
	_: NextHandler<EveContext, Error>,
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

	log::trace!("request_id: {} - Deleting database cluster", request_id);
	service::delete_managed_database(
		context.get_database_connection(),
		&database_id,
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

	context.success(DeleteDatabaseResponse {}).await?;
	Ok(context)
}

async fn change_database_password(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	let config = context.get_state().config.clone();

	let ChangeDatabasePasswordRequest { new_password, .. } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let database_id =
		Uuid::parse_str(context.get_param(request_keys::DATABASE_ID).unwrap())
			.unwrap();

	service::change_database_password(
		context.get_database_connection(),
		&database_id,
		&request_id,
		&new_password,
		&config,
	)
	.await?;

	context.success(ChangeDatabasePasswordResponse {}).await?;
	Ok(context)
}
