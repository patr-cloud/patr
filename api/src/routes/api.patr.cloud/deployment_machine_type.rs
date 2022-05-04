use api_models::models::deployment_machine_type::{
	DeploymentMachineType,
	ListAllDeploymentMachineTypeResponse,
};
use eve_rs::{App as EveApp, NextHandler};

use crate::{
	app::{create_eve_app, App},
	db,
	pin_fn,
	utils::{Error, ErrorData, EveContext, EveMiddleware},
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
	let deployment_machine_types =
		db::get_all_deployment_machine_types(context.get_database_connection())
			.await?
			.into_iter()
			.map(|machine_type| DeploymentMachineType {
				id: machine_type.id,
				cpu_count: machine_type.cpu_count,
				memory_count: machine_type.memory_count,
			})
			.collect();
	context.success(ListAllDeploymentMachineTypeResponse {
		deployment_machine_types,
	});

	Ok(context)
}
