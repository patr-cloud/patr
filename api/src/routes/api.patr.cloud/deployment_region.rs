use api_models::models::deployment_region::{
	DeploymentRegion,
	ListAllDeploymentRegionResponse,
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
			EveMiddleware::CustomFunction(pin_fn!(get_all_deployment_regions)),
		],
	);

	app
}

async fn get_all_deployment_regions(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let deployment_regions =
		db::get_all_deployment_regions(context.get_database_connection())
			.await?
			.into_iter()
			.filter_map(|region| {
				Some(DeploymentRegion {
					id: region.id,
					name: region.name,
					provider: region.cloud_provider?.to_string(),
				})
			})
			.collect();
	context.success(ListAllDeploymentRegionResponse { deployment_regions });

	Ok(context)
}
