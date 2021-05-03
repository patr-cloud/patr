use api_macros::closure_as_pinned_box;
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use hex::ToHex;
use serde_json::json;

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac::{self, permissions},
	pin_fn,
	utils::{
		constants::request_keys,
		validator,
		Error,
		ErrorData,
		EveContext,
		EveMiddleware,
	},
};

pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut app = create_eve_app(&app);

	// create new repository
	app.post(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::docker_registry::CREATE,
				closure_as_pinned_box!(|mut context| {
					let org_id_string = context
						.get_param(request_keys::ORGANISATION_ID)
						.unwrap();
					let organisation_id = hex::decode(&org_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

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

	app.get(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::docker_registry::LIST,
				closure_as_pinned_box!(|mut context| {
					let org_id_string = context
						.get_param(request_keys::ORGANISATION_ID)
						.unwrap();
					let organisation_id = hex::decode(&org_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

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
			EveMiddleware::CustomFunction(pin_fn!(list_docker_repositories)),
		],
	);

	app.delete(
		"/:repositoryId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::docker_registry::DELETE,
				closure_as_pinned_box!(|mut context| {
					let repo_id_string =
						context.get_param(request_keys::REPOSITORY_ID).unwrap();
					let repository_id = hex::decode(&repo_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_mysql_connection(),
						&repository_id,
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
			EveMiddleware::CustomFunction(pin_fn!(delete_docker_repository)),
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
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	// check if the token is valid
	let body = context.get_body_object().clone();
	let repository = body
		.get(request_keys::REPOSITORY)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	// check if repo name is valid
	let is_repo_name_valid = validator::is_docker_repo_name_valid(&repository);
	if !is_repo_name_valid {
		context.status(400).json(error!(INVALID_REPOSITORY_NAME));
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
		Error::as_result()
			.status(400)
			.body(error!(RESOURCE_EXISTS).to_string())?;
	}

	// split the repo nam in 2 halfs, and validate org, and repo name
	let resource_id =
		db::generate_new_resource_id(context.get_mysql_connection()).await?;
	let resource_id = resource_id.as_bytes();

	// safe to assume that org id is present here
	let organisation_id =
		context.get_param(request_keys::ORGANISATION_ID).unwrap();
	let organisation_id = hex::decode(&organisation_id).unwrap();

	// call function to add repo details to the table
	// `docker_registry_repository` add a new resource
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

async fn list_docker_repositories(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let org_id_string =
		context.get_param(request_keys::ORGANISATION_ID).unwrap();
	let organisation_id = hex::decode(&org_id_string).unwrap();

	let repositories = db::get_docker_repositories_for_organisation(
		context.get_mysql_connection(),
		&organisation_id,
	)
	.await?
	.into_iter()
	.map(|repository| {
		json!({
			request_keys::ID: repository.id.encode_hex::<String>(),
			request_keys::NAME: repository.name,
		})
	})
	.collect::<Vec<_>>();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::REPOSITORIES: repositories
	}));

	Ok(context)
}

async fn delete_docker_repository(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let repo_id_string =
		context.get_param(request_keys::REPOSITORY_ID).unwrap();
	let repository_id = hex::decode(&repo_id_string).unwrap();

	db::get_docker_repository_by_id(
		context.get_mysql_connection(),
		&repository_id,
	)
	.await?
	.status(200)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	// TODO delete from docker registry using its API

	db::delete_docker_repository_by_id(
		context.get_mysql_connection(),
		&repository_id,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
	}));
	Ok(context)
}
