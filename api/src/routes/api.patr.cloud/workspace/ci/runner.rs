use api_macros::closure_as_pinned_box;
use api_models::{
	models::workspace::ci::runner::{
		CreateRunnerRequest,
		CreateRunnerResponse,
		DeleteRunnerResponse,
		GetRunnerInfoResponse,
		ListCiRunnerBuildHistoryResponse,
		ListCiRunnerResponse,
		UpdateRunnerRequest,
		UpdateRunnerResponse,
	},
	utils::Uuid,
};
use axum::{
	routing::{delete, get, patch, post},
	Router,
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};

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

pub fn create_sub_route(app: &App) -> Router {
	let router = Router::new();

	router.route("/", get(list_runner));

	router.route("/", post(create_runner));

	router.route("/:runnerId", get(get_runner_info));

	router.route("/:runnerId/history", get(list_build_details_for_runner));

	router.route("/:runnerId", patch(update_runner));

	router.route("/:runnerId", delete(delete_runner));

	router
}

async fn list_runner(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	log::trace!(
		"request_id: {} - Getting list of ci runner for workspace {}",
		request_id,
		workspace_id
	);

	let runners = db::get_runners_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.into_iter()
	.map(Into::into)
	.collect();

	log::trace!(
		"request_id: {} - Returning list of ci runner for workspace {}",
		request_id,
		workspace_id
	);

	context.success(ListCiRunnerResponse { runners });
	Ok(context)
}

async fn create_runner(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let CreateRunnerRequest {
		workspace_id: _,
		name,
		region_id,
		build_machine_type_id,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	log::trace!(
		"request_id: {} - Creating ci runner for workspace {}",
		request_id,
		workspace_id
	);

	let id = service::create_runner_for_workspace(
		context.get_database_connection(),
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

	context.success(CreateRunnerResponse { id });
	Ok(context)
}

async fn get_runner_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let runner_id =
		Uuid::parse_str(context.get_param(request_keys::RUNNER_ID).unwrap())
			.unwrap();

	log::trace!(
		"request_id: {} - Getting info of ci runner for workspace {}",
		request_id,
		workspace_id
	);

	let runner =
		db::get_runner_by_id(context.get_database_connection(), &runner_id)
			.await?
			.map(Into::into)
			.status(404)
			.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	log::trace!(
		"request_id: {} - Returning info of ci runner for workspace {}",
		request_id,
		workspace_id
	);

	context.success(GetRunnerInfoResponse(runner));
	Ok(context)
}

async fn list_build_details_for_runner(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let runner_id =
		Uuid::parse_str(context.get_param(request_keys::RUNNER_ID).unwrap())
			.unwrap();

	log::trace!(
		"request_id: {} - Getting history of ci runner for workspace {}",
		request_id,
		workspace_id
	);

	let builds = db::list_build_details_for_runner(
		context.get_database_connection(),
		&runner_id,
	)
	.await?;

	log::trace!(
		"request_id: {} - Returning history of ci runner for workspace {}",
		request_id,
		workspace_id
	);

	context.success(ListCiRunnerBuildHistoryResponse { builds });
	Ok(context)
}

async fn update_runner(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let runner_id =
		Uuid::parse_str(context.get_param(request_keys::RUNNER_ID).unwrap())
			.unwrap();

	let UpdateRunnerRequest {
		workspace_id: _,
		runner_id: _,
		name,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	log::trace!(
		"request_id: {} - Update ci runner for workspace {}",
		request_id,
		workspace_id
	);

	service::update_runner(
		context.get_database_connection(),
		&runner_id,
		&name,
		&request_id,
	)
	.await?;

	log::trace!(
		"request_id: {} - Updated ci runner for workspace {}",
		request_id,
		workspace_id
	);

	context.success(UpdateRunnerResponse {});
	Ok(context)
}

async fn delete_runner(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let runner_id =
		Uuid::parse_str(context.get_param(request_keys::RUNNER_ID).unwrap())
			.unwrap();

	log::trace!(
		"request_id: {} - Delete ci runner for workspace {}",
		request_id,
		workspace_id
	);

	service::delete_runner(
		context.get_database_connection(),
		&runner_id,
		&request_id,
	)
	.await?;

	log::trace!(
		"request_id: {} - Deleted ci runner for workspace {}",
		request_id,
		workspace_id
	);

	context.success(DeleteRunnerResponse {});
	Ok(context)
}
