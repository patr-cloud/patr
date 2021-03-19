use api_macros::closure_as_pinned_box;
use eve_rs::{App as EveApp, Context, Error, NextHandler};

use crate::{
	app::{create_eve_app, App},
	db, error,
	models::rbac::{self, permissions},
	pin_fn,
	utils::{constants::request_keys, EveContext, EveMiddleware},
};
use serde_json::{json, Value};

pub fn create_sub_app(app: &App) -> EveApp<EveContext, EveMiddleware, App> {
	let mut app = create_eve_app(&app);

	// List all deployments
	app.get(
		"/",
		&[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::deployer::LIST,
				closure_as_pinned_box!(|mut context| {
					let org_id_string = context
						.get_param(request_keys::ORGANISATION_ID)
						.unwrap();
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
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(list_deployments)),
		],
	);

	app.post(
		"/create-repository",
		&[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::deployer::CREATE,
				closure_as_pinned_box!(|mut context| {
					let org_id_string = context
						.get_param(request_keys::ORGANISATION_ID)
						.unwrap();
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
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(create_docker_repository)),
		],
	);
	app
}

async fn list_deployments(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let _body_object = context.get_body_object().clone();

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

// middleware to create a new docker repository
// add details regarding the repository to the db and give user push and pull access
// possible body for repository
// {
// 	"repoName"
// }
async fn create_docker_repository(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	// check if the token is valid
	let body = context.get_body_object().clone();
	let repository_name = if let Some(Value::String(repository_name)) =
		body.get(request_keys::REPOSITORY_NAME)
	{
		repository_name
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let resource_id =
		db::generate_new_resource_id(context.get_mysql_connection()).await?;
	let resource_id = resource_id.as_bytes();

	// safe to assume that org id is present here
	let organisation_id =
		context.get_param(request_keys::ORGANISATION_ID).unwrap();
	let organisation_id = hex::decode(&organisation_id).unwrap();

	// call function to add repo details to the table `docker_registry_repository`
	// add a new resource
	db::create_resource(
		context.get_mysql_connection(),
		resource_id,
		&repository_name,
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::DOCKER_REPOSITORY)
			.unwrap(),
		&organisation_id,
	)
	.await?;
	db::add_repository(
		context.get_mysql_connection(),
		resource_id,
		&repository_name,
		&organisation_id,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
	}));

	Ok(context)
}
