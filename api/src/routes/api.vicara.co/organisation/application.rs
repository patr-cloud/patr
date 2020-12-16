
use eve_rs::{App as EveApp, Context, Error, NextHandler};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac::permissions,
	utils::{constants::request_keys, EveContext, EveMiddleware},
	pin_fn,
};
use serde_json::{json, Value};


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
		),
		EveMiddleware::CustomFunction(pin_fn!(
			get_applications_for_organisation
		)),
		
		],
	);


	// get details for an application
	// app.get(
	// 	"/:applicationId",
	// 	&[
	// 		EveMiddleware::ResourceTokenAuthenticator(

	// 		)
	// 	]
	// )
	app
}


// todo: write a function which will list out all the applications.
// write a function which will show information regarding a specific application.



/**
 * Function to list out all the application in an organisation.
 */
async fn get_applications_for_organisation(
	mut context : EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	
	// maybe take this
	let organisation_id =
		hex::decode(context.get_param(request_keys::ORGANISATION_ID).unwrap())
			.unwrap();

	
	let applications = db::get_applications_for_organisation(
		context.get_mysql_connection()
	)
	.await?
	.into_iter()
	.map(|application| {
		let id = hex::encode(application.id); // get application id 
		json!({
				request_keys::ID : id,
				request_keys::NAME : application.name,
			})

	})
	.collect::<Vec<_>>();

	context.json(json!({	
		request_keys::SUCCESS : true,
		request_keys::APPLICATIONS: applications,
	}));

	Ok(context)

}