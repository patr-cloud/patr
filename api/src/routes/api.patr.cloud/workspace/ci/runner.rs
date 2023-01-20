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

pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut sub_app = create_eve_app(app);

	sub_app.get(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: false,
				permission: permissions::workspace::ci::runner::LIST,
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
			EveMiddleware::CustomFunction(pin_fn!(list_runner)),
		],
	);

	sub_app.post(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: false,
				permission: permissions::workspace::ci::runner::CREATE,
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
			EveMiddleware::CustomFunction(pin_fn!(create_runner)),
		],
	);

	sub_app.get(
		"/:runner_id",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: false,
				permission: permissions::workspace::ci::runner::INFO,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let runner_id =
						context.get_param(request_keys::RUNNER_ID).unwrap();
					let runner_id = Uuid::parse_str(runner_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&runner_id,
					)
					.await?
					.filter(|resource| resource.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(get_runner_info)),
		],
	);

	sub_app.get(
		"/:runner_id/history",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: false,
				permission: permissions::workspace::ci::runner::INFO,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let runner_id =
						context.get_param(request_keys::RUNNER_ID).unwrap();
					let runner_id = Uuid::parse_str(runner_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&runner_id,
					)
					.await?
					.filter(|resource| resource.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(
				list_build_details_for_runner
			)),
		],
	);

	sub_app.patch(
		"/:runner_id",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: false,
				permission: permissions::workspace::ci::runner::UPDATE,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let runner_id =
						context.get_param(request_keys::RUNNER_ID).unwrap();
					let runner_id = Uuid::parse_str(runner_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&runner_id,
					)
					.await?
					.filter(|resource| resource.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(update_runner)),
		],
	);

	sub_app.delete(
		"/:runner_id",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: false,
				permission: permissions::workspace::ci::runner::DELETE,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let runner_id =
						context.get_param(request_keys::RUNNER_ID).unwrap();
					let runner_id = Uuid::parse_str(runner_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&runner_id,
					)
					.await?
					.filter(|resource| resource.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(delete_runner)),
		],
	);

	sub_app
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
