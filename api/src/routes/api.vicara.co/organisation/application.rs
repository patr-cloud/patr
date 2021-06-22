use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use hex::ToHex;
use serde_json::json;

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac::permissions,
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

	// List all applications
	app.get(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::application::LIST,
				api_macros::closure_as_pinned_box!(|mut context| {
					let org_id_string = context
						.get_param(request_keys::ORGANISATION_ID)
						.unwrap();
					let organisation_id = hex::decode(&org_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&organisation_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(get_applications)),
		],
	);

	// get details for an application
	app.get(
		"/:applicationId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::application::VIEW_DETAILS,
				api_macros::closure_as_pinned_box!(|mut context| {
					let application_id_string = context
						.get_param(request_keys::APPLICATION_ID)
						.unwrap();
					let application_id = hex::decode(&application_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&application_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(
				get_application_info_in_organisation
			)),
		],
	);

	// get list of versions for an application
	app.get(
		"/:applicationId/versions",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::application::LIST_VERSIONS,
				api_macros::closure_as_pinned_box!(|mut context| {
					let application_id_string = context
						.get_param(request_keys::APPLICATION_ID)
						.unwrap();
					let application_id = hex::decode(&application_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					// check if resource with the given application id exists.
					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&application_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(
				get_all_versions_for_application
			)),
		],
	);

	app
}

/// Function to list out all the application in an organisation.
async fn get_applications(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let organisation_id =
		hex::decode(context.get_param(request_keys::ORGANISATION_ID).unwrap())
			.unwrap();

	let applications = db::get_applications_in_organisation(
		context.get_database_connection(),
		&organisation_id,
	)
	.await?
	.into_iter()
	.map(|application| {
		let id = application.id.encode_hex::<String>(); // get application id
		json!({
			request_keys::ID : id,
			request_keys::NAME : application.name,
		})
	})
	.collect::<Vec<_>>();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::APPLICATIONS: applications,
	}));
	Ok(context)
}

/// get details for an application
async fn get_application_info_in_organisation(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let application_id =
		context.get_param(request_keys::APPLICATION_ID).unwrap();
	let application_id = hex::decode(application_id).unwrap();
	let application = db::get_application_by_id(
		context.get_database_connection(),
		&application_id,
	)
	.await?;

	// since the resource/application is already been checked in
	// ResourceTokenAuthenticator, application cannot be null, else, the code
	// would not reach at this point Hence, it is safe to unwrap the
	// application.
	let application = application.unwrap();

	// add response to context json
	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::APPLICATION_ID: application_id,
		request_keys::NAME: application.name,
	}));
	Ok(context)
}

/// List out all the versions of an application.
async fn get_all_versions_for_application(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let application_id_string = context
		.get_param(request_keys::APPLICATION_ID)
		.unwrap()
		.clone();

	// it is safe to unwrap as the a resource for the given application already
	// exists.
	let application_id = hex::decode(&application_id_string).unwrap();

	// call fetch query for the given application id.
	let versions = db::get_all_versions_for_application(
		context.get_database_connection(),
		&application_id,
	)
	.await?
	.into_iter()
	.map(|version| {
		json!({
			request_keys::APPLICATION_ID: application_id,
			request_keys::VERSION : version.version
		})
	})
	.collect::<Vec<_>>();

	// send true, application id, and versions.
	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::APPLICATION_ID: application_id_string,
		request_keys::VERSIONS: versions
	}));
	Ok(context)
}
