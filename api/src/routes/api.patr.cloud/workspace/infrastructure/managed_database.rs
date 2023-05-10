use api_models::{
	models::prelude::{
		CreateManagedDatabasePath,
		CreateManagedDatabaseRequest,
		CreateManagedDatabaseResponse,
		DeleteManagedDatabasePath,
		DeleteManagedDatabaseRequest,
		GetManagedDatabasePath,
		GetManagedDatabaseRequest,
		GetManagedDatabaseResponse,
		ListAllManagedDatabasePath,
		ListAllManagedDatabaseRequest,
		ListAllManagedDatabaseResponse,
		ManagedDatabase,
		ManagedDatabaseConnection,
	},
	prelude::*,
	utils::Uuid,
};
use axum::{extract::State, Extension, Router};

use crate::{
	app::App,
	db,
	models::{rbac::permissions, ResourceType, UserAuthenticationData},
	prelude::*,
	service,
	utils::Error,
};

pub fn create_sub_app(app: &App) -> Router<App> {
	Router::new()
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::managed_database::LIST,
				|ListAllManagedDatabasePath { workspace_id },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id)
						.await
						.filter(|value| value.owner_id == workspace_id);
				},
			),
			app.clone(),
			list_all_database_clusters,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::managed_database::CREATE,
				|CreateManagedDatabasePath { workspace_id },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id)
						.await
						.filter(|value| value.owner_id == workspace_id);
				},
			), app.clone(), create_database_cluster)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::managed_database::LIST,
				|GetManagedDatabasePath { workspace_id, database_id },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &database_id)
						.await
						.filter(|value| value.owner_id == workspace_id);
				},
			), app.clone(), get_managed_database_info)
}

async fn list_all_database_clusters(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: ListAllManagedDatabasePath { workspace_id },
		query: (),
		body: (),
	}: DecodedRequest<ListAllManagedDatabaseRequest>,
) -> Result<ListAllManagedDatabaseResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!(
		"request_id: {} - Getting all database cluster info from db",
		request_id
	);
	let database_clusters = db::get_all_database_clusters_for_workspace(
		&mut connection,
		&workspace_id,
	)
	.await?
	.into_iter()
	.map(|database| ManagedDatabase {
		id: database.id,
		name: database.name,
		database_name: database.db_name,
		engine: database.engine.to_string(),
		version: database.version,
		num_nodes: database.num_nodes,
		database_plan: database.database_plan.to_string(),
		region: database.region,
		status: database.status.to_string(),
		public_connection: ManagedDatabaseConnection {
			host: database.host,
			port: database.port,
			username: database.username,
			password: database.password,
		},
	})
	.collect::<Vec<_>>();

	log::trace!(
		"request_id: {} - Returning all database cluster info",
		request_id
	);

	Ok(ListAllManagedDatabaseResponse {
		databases: database_clusters,
	})
}

async fn create_database_cluster(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: CreateManagedDatabasePath { workspace_id },
		query: (),
		body:
			CreateManagedDatabaseRequest {
				name,
				db_name,
				version,
				engine,
				num_nodes,
				database_plan,
				region,
			},
	}: DecodedRequest<CreateManagedDatabaseRequest>,
) -> Result<CreateManagedDatabaseResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {} - Creating database cluster", request_id);

	let database_id = service::create_managed_database_in_workspace(
		&mut connection,
		&name,
		&db_name,
		&engine,
		version,
		num_nodes,
		&database_plan,
		&region,
		&workspace_id,
		&config,
		&request_id,
	)
	.await?;

	let _ = service::get_internal_metrics(
		&mut connection,
		"A database instance has been created",
	)
	.await;

	Ok(CreateManagedDatabaseResponse { id: database_id })
}

async fn get_managed_database_info(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: GetManagedDatabasePath {
			workspace_id,
			database_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<GetManagedDatabaseRequest>,
) -> Result<GetManagedDatabaseResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {} - Getting database info", request_id);
	let database =
		db::get_managed_database_by_id(&mut connection, &database_id)
			.await?
			.map(|database| ManagedDatabase {
				id: database.id,
				name: database.name,
				database_name: database.db_name,
				engine: database.engine.to_string(),
				version: database.version,
				num_nodes: database.num_nodes,
				database_plan: database.database_plan.to_string(),
				region: database.region,
				status: database.status.to_string(),
				public_connection: ManagedDatabaseConnection {
					host: database.host,
					port: database.port,
					username: database.username,
					password: database.password,
				},
			})
			.ok_or_else(|| ErrorType::WrongParameters)?;

	log::trace!("request_id: {} - Returning database info", request_id);

	Ok(GetManagedDatabaseResponse { database })
}

async fn delete_managed_database(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: DeleteManagedDatabasePath {
			workspace_id,
			database_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<DeleteManagedDatabaseRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();

	let user_id = token_data.user_id().clone();

	let database =
		db::get_managed_database_by_id(&mut connection, &database_id)
			.await?
			.ok_or_else(|| ErrorType::NotFound)?;

	log::trace!("request_id: {} - Deleting database cluster", request_id);
	service::delete_managed_database(
		&mut connection,
		&database_id,
		&config,
		&request_id,
	)
	.await?;

	// Commiting transaction so that even if the mailing function fails the
	// resource should be deleted
	connection.commit().await?;

	service::resource_delete_action_email(
		&mut connection,
		&database.name,
		&database.workspace_id,
		&ResourceType::ManagedDatabase,
		&user_id,
	)
	.await?;

	service::get_internal_metrics(
		&mut connection,
		"A database instance has been deleted",
	)
	.await;

	Ok(())
}
