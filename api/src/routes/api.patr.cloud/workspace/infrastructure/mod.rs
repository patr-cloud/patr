use api_models::{
	models::workspace::infrastructure::{
		database::{DatabasePlanType, ListAllDatabasePlanResponse},
		list_all_deployment_machine_type::{
			DeploymentMachineType,
			ListAllDeploymentMachineTypesResponse,
		},
	},
	utils::Uuid,
};
use eve_rs::{App as EveApp, AsError, NextHandler};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac,
	pin_fn,
	utils::{constants::request_keys, Error, EveContext, EveMiddleware},
};

mod deployment;
mod managed_database;
mod managed_url;
mod static_site;

pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, Error> {
	let mut sub_app = create_eve_app(app);

	sub_app.use_sub_app("/deployment", deployment::create_sub_app(app));
	sub_app.use_sub_app(
		"/managed-database",
		managed_database::create_sub_app(app),
	);
	sub_app.use_sub_app("/managed-url", managed_url::create_sub_app(app));
	sub_app.use_sub_app("/static-site", static_site::create_sub_app(app));

	sub_app.get(
		"/machine-type",
		[
			EveMiddleware::PlainTokenAuthenticator {
				is_api_token_allowed: true,
			},
			EveMiddleware::CustomFunction(pin_fn!(
				get_all_deployment_machine_types
			)),
		],
	);

	sub_app.get(
		"/database-plan",
		[
			EveMiddleware::PlainTokenAuthenticator {
				is_api_token_allowed: true,
			},
			EveMiddleware::CustomFunction(pin_fn!(get_all_database_plans)),
		],
	);

	sub_app
}

async fn get_all_deployment_machine_types(
	mut context: EveContext,
	_: NextHandler<EveContext, Error>,
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

	let machine_types =
		db::get_all_deployment_machine_types(context.get_database_connection())
			.await?
			.into_iter()
			.map(|machine_type| DeploymentMachineType {
				id: machine_type.id,
				cpu_count: machine_type.cpu_count,
				memory_count: machine_type.memory_count,
			})
			.collect();

	context
		.success(ListAllDeploymentMachineTypesResponse { machine_types })
		.await?;
	Ok(context)
}

async fn get_all_database_plans(
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

	// Validate if the user belongs to the workspace or not
	if !access_token_data
		.workspace_permissions()
		.contains_key(&workspace_id) &&
		access_token_data.user_id() != god_user_id
	{
		Error::as_result()
			.status(404)
			.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;
	}

	let database_plans =
		db::get_all_database_plans(context.get_database_connection())
			.await?
			.into_iter()
			.map(|plan| DatabasePlanType {
				id: plan.id,
				cpu_count: plan.cpu_count,
				memory_count: plan.memory_count,
				volume: plan.volume,
			})
			.collect();

	context.success(ListAllDatabasePlanResponse { database_plans });
	Ok(context)
}
