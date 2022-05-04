use api_models::{
	models::workspace::infrastructure::list_all_deployment_machine_type::{
		DeploymentMachineType,
		ListAllDeploymentMachineTypesResponse,
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
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(
				get_all_deployment_machine_types
			)),
		],
	);

	app
}

async fn get_all_deployment_machine_types(
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

	if !access_token_data.workspaces.contains_key(&workspace_id) &&
		&access_token_data.user.id != god_user_id
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
	context.success(ListAllDeploymentMachineTypesResponse { machine_types });

	Ok(context)
}
