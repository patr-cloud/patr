mod git_provider;
mod runner;

use api_macros::closure_as_pinned_box;
use api_models::{
	models::workspace::ci::{
		get_recent_activity::GetRecentActivityResponse,
		list_all_build_machine_type::ListAllBuildMachineTypesResponse,
	},
	utils::Uuid,
};
use axum::{routing::get, Router};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac::{self, permissions},
	pin_fn,
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

	router.nest("/git-provider", git_provider::create_sub_route(app));
	router.nest("/runner", runner::create_sub_route(app));

	// This route implement PlainTokenAuthenticator
	router.route("/build-machine-type", get(get_all_build_machine_types));

	router.route("/recent-activity", get(get_recent_activity_for_ci));

	router
}

async fn get_all_build_machine_types(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id = context
		.get_param(request_keys::WORKSPACE_ID)
		.and_then(|workspace_id| Uuid::parse_str(workspace_id).ok())
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let access_token_data = context.get_token_data().unwrap();
	let god_user_id = rbac::GOD_USER_ID.get().unwrap();

	if !access_token_data
		.workspace_permissions()
		.contains_key(&workspace_id) &&
		access_token_data.user_id() != god_user_id
	{
		Error::as_result()
			.status(404)
			.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;
	}

	let build_machine_types =
		db::get_all_build_machine_types(context.get_database_connection())
			.await?;

	context.success(ListAllBuildMachineTypesResponse {
		build_machine_types,
	});
	Ok(context)
}

async fn get_recent_activity_for_ci(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	log::trace!("request_id: {request_id} - Listing recent activity for ci",);

	let activity = db::get_recent_activity_for_ci_in_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?;

	context.success(GetRecentActivityResponse {
		activities: activity,
	});
	Ok(context)
}
