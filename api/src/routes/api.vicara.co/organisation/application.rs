use eve_rs::{App as EveApp, Context};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac::permissions,
	utils::{constants::request_keys, EveContext, EveMiddleware},
};

pub fn create_sub_app(app: &App) -> EveApp<EveContext, EveMiddleware, App> {
	let mut app = create_eve_app(&app);

	// List all applications
	app.get(
		"/",
		&[EveMiddleware::ResourceTokenAuthenticator(
			permissions::organisation::application::LIST,
			api_macros::closure_as_pinned_box!(|mut context| {
				let org_id_string =
					context.get_param(request_keys::ORGANISATION_ID).unwrap();
				let organisation_id = hex::decode(&org_id_string);
				if organisation_id.is_err() {
					context.status(400).json(error!(WRONG_PARAMETERS));
					return Ok((context, None));
				}
				let organisation_id = organisation_id.unwrap();

				let resource = db::get_resource_by_id(
					context.get_mysql_connection(),
					&organisation_id,
				)
				.await?;

				if resource.is_none() {
					context.status(404).json(error!(RESOURCE_DOES_NOT_EXIST));
				}

				Ok((context, resource))
			}),
		)],
	);

	app
}
