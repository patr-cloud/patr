use api_macros::closure_as_pinned_box;
use eve_rs::{App as EveApp, Context, Error, NextHandler};
use validator::is_docker_repo_name_valid;

use crate::{
	app::{create_eve_app, App},
	db, error,
	models::rbac::{self, permissions},
	pin_fn,
	utils::{constants::request_keys, validator, EveContext, EveMiddleware},
};
use serde_json::{json, Value};

pub fn create_sub_app(app: &App) -> EveApp<EveContext, EveMiddleware, App> {
	let mut app = create_eve_app(&app);

	// create new repository
	app.post(
		"/",
		&[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::docker_registry::CREATE,
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

// middleware to create a new docker repository
// possible request body to create repository
// {
// 	"repoName"
// }
async fn create_docker_repository(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	// check if the token is valid
	let body = context.get_body_object().clone();
	let repository = if let Some(Value::String(repository)) =
		body.get(request_keys::REPOSITORY)
	{
		repository
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	// check if repo name is valid
	let is_repo_name_valid = is_docker_repo_name_valid(&repository);
	if !is_repo_name_valid {
		context.json(json!({
			request_keys::SUCCESS: false,
			request_keys::MESSAGE: "invalid repository name."
		}));

		return Ok(context);
	}

	let org_id_string =
		context.get_param(request_keys::ORGANISATION_ID).unwrap();
	let organisation_id = hex::decode(&org_id_string).unwrap();

	// check if repository already exists
	let check = db::get_repository_by_name(
		context.get_mysql_connection(),
		&repository,
		&organisation_id,
	)
	.await?;

	if check.is_some() {
		context.status(400).json(error!(REPOSITORY_ALREADY_EXISTS));
		return Ok(context);
	}

	// split the repo nam in 2 halfs, and validate org, and repo name
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
		&repository,
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::DOCKER_REPOSITORY)
			.unwrap(),
		&organisation_id,
	)
	.await?;

	db::create_repository(
		context.get_mysql_connection(),
		resource_id,
		&repository,
		&organisation_id,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ID: resource_id
	}));

	Ok(context)
}
