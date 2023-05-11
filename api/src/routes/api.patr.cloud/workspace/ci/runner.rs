use api_models::{models::prelude::*, utils::Uuid};
use axum::{extract::State, Router};

use crate::{
	app::App,
	db,
	models::rbac::permissions,
	prelude::*,
	service,
	utils::Error,
};

pub fn create_sub_app(app: &App) -> Router<App> {
	Router::new()
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::ci::runner::LIST,
				|ListCiRunnerPath { workspace_id }, (), app, request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			)
			.disallow_api_token(),
			app.clone(),
			list_runner,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::ci::runner::CREATE,
				|CreateRunnerPath { workspace_id }, (), app, request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			),
			app.clone(),
			create_runner,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::ci::runner::INFO,
				|GetRunnerInfoPath {
				     workspace_id,
				     runner_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &runner_id).await
				},
			)
			.disallow_api_token(),
			app.clone(),
			get_runner_info,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::ci::runner::LIST,
				|ListCiRunnerBuildHistoryPath {
				     workspace_id,
				     runner_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &runner_id).await
				},
			)
			.disallow_api_token(),
			app.clone(),
			list_build_details_for_runner,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::ci::runner::UPDATE,
				|UpdateRunnerPath {
				     workspace_id,
				     runner_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &runner_id).await
				},
			)
			.disallow_api_token(),
			app.clone(),
			update_runner,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::ci::runner::DELETE,
				|DeleteRunnerPath {
				     workspace_id,
				     runner_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &runner_id).await
				},
			)
			.disallow_api_token(),
			app.clone(),
			delete_runner,
		)
}

async fn list_runner(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: ListCiRunnerPath { workspace_id },
		query: (),
		body: (),
	}: DecodedRequest<ListCiRunnerRequest>,
) -> Result<ListCiRunnerResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!(
		"request_id: {} - Getting list of ci runner for workspace {}",
		request_id,
		workspace_id
	);

	let runners = db::get_runners_for_workspace(&mut connection, &workspace_id)
		.await?
		.into_iter()
		.map(Into::into)
		.collect();

	log::trace!(
		"request_id: {} - Returning list of ci runner for workspace {}",
		request_id,
		workspace_id
	);

	Ok(ListCiRunnerResponse { runners })
}

async fn create_runner(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: CreateRunnerPath { workspace_id },
		query: (),
		body:
			CreateRunnerRequest {
				name,
				region_id,
				build_machine_type_id,
			},
	}: DecodedRequest<CreateRunnerRequest>,
) -> Result<CreateRunnerResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!(
		"request_id: {} - Creating ci runner for workspace {}",
		request_id,
		workspace_id
	);

	let id = service::create_runner_for_workspace(
		&mut connection,
		&workspace_id,
		&name,
		&region_id,
		&build_machine_type_id,
		&request_id,
	)
	.await?;

	log::trace!(
		"request_id: {} - Created of ci runner for workspace {}",
		request_id,
		workspace_id
	);

	Ok(CreateRunnerResponse { id });
}

async fn get_runner_info(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: GetRunnerInfoPath {
			workspace_id,
			runner_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<GetRunnerInfoRequest>,
) -> Result<GetRunnerInfoResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!(
		"request_id: {} - Getting info of ci runner for workspace {}",
		request_id,
		workspace_id
	);

	let runner = db::get_runner_by_id(&mut connection, &runner_id)
		.await?
		.map(Into::into)
		.ok_or_else(|| ErrorType::NotFound)?;

	log::trace!(
		"request_id: {} - Returning info of ci runner for workspace {}",
		request_id,
		workspace_id
	);

	Ok(GetRunnerInfoResponse(runner));
}

async fn list_build_details_for_runner(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: ListCiRunnerBuildHistoryPath {
			workspace_id,
			runner_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<ListCiRunnerBuildHistoryRequest>,
) -> Result<ListCiRunnerBuildHistoryResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!(
		"request_id: {} - Getting history of ci runner for workspace {}",
		request_id,
		workspace_id
	);

	let builds =
		db::list_build_details_for_runner(&mut connection, &runner_id).await?;

	log::trace!(
		"request_id: {} - Returning history of ci runner for workspace {}",
		request_id,
		workspace_id
	);

	Ok(ListCiRunnerBuildHistoryResponse { builds });
}

async fn update_runner(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: UpdateRunnerPath {
			workspace_id,
			runner_id,
		},
		query: (),
		body: UpdateRunnerRequest { name },
	}: DecodedRequest<UpdateRunnerRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();

	log::trace!(
		"request_id: {} - Update ci runner for workspace {}",
		request_id,
		workspace_id
	);

	service::update_runner(&mut connection, &runner_id, &name, &request_id)
		.await?;

	log::trace!(
		"request_id: {} - Updated ci runner for workspace {}",
		request_id,
		workspace_id
	);

	Ok(());
}

async fn delete_runner(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: DeleteRunnerPath {
			workspace_id,
			runner_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<DeleteRunnerRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();

	log::trace!(
		"request_id: {} - Delete ci runner for workspace {}",
		request_id,
		workspace_id
	);

	service::delete_runner(&mut connection, &runner_id, &request_id).await?;

	log::trace!(
		"request_id: {} - Deleted ci runner for workspace {}",
		request_id,
		workspace_id
	);

	Ok(());
}
