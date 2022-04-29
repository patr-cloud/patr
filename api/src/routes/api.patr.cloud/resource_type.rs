use api_models::models::resource_type::{
	ListAllResourceTypeResponse,
	ResourceType,
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
			EveMiddleware::CustomFunction(pin_fn!(get_all_resource_types)),
		],
	);

	app
}

async fn get_all_resource_types(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let resource_types =
		db::get_all_resource_types(context.get_database_connection())
			.await?
			.into_iter()
			.map(|resource_type| ResourceType {
				id: resource_type.id,
				name: resource_type.name,
				description: resource_type.description,
			})
			.collect();
	context.success(ListAllResourceTypeResponse { resource_types });

	Ok(context)
}
