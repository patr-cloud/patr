use api_models::models::permission::{ListAllPermissionsResponse, Permission};
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
			EveMiddleware::CustomFunction(pin_fn!(get_all_permissions)),
		],
	);

	app
}

async fn get_all_permissions(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let permissions =
		db::get_all_permissions(context.get_database_connection())
			.await?
			.into_iter()
			.map(|permission| Permission {
				id: permission.id,
				name: permission.name,
				description: permission.description,
			})
			.collect();
	context.success(ListAllPermissionsResponse { permissions });

	Ok(context)
}
