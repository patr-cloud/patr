use api_macros::closure_as_pinned_box;
use api_models::models::workspace::docker_registry::{
	CreateDockerRepositoryRequest,
	CreateDockerRepositoryResponse,
	DeleteDockerRepositoryImageResponse,
	DeleteDockerRepositoryResponse,
	DockerRepository,
	DockerRepositoryTagAndDigestInfo,
	GetDockerRepositoryImageDetailsResponse,
	GetDockerRepositoryInfoResponse,
	GetDockerRepositoryTagDetailsResponse,
	ListDockerRepositoriesResponse,
	ListDockerRepositoryTagsResponse,
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use uuid::Uuid;

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac::{self, permissions},
	pin_fn,
	service,
	utils::{
		constants::request_keys,
		get_current_time_millis,
		validator,
		Error,
		ErrorData,
		EveContext,
		EveMiddleware,
	},
};

/// # Description
/// This function is used to create a sub app for every endpoint listed. It
/// creates an eve app which binds the endpoint with functions.
///
/// # Arguments
/// * `app` - an object of type [`App`] which contains all the configuration of
///   api including the
/// database connections.
///
/// # Returns
/// this function returns `EveApp<EveContext, EveMiddleware, App, ErrorData>`
/// containing context, middleware, object of [`App`] and Error
///
/// [`App`]: App
pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut app = create_eve_app(app);

	// create new repository
	app.post(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::docker_registry::CREATE,
				closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = hex::decode(&workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
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

	// Get list of repositories
	app.get(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::docker_registry::LIST,
				closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = hex::decode(&workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
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

	// Get repository info
	app.get(
		"/:repositoryId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::docker_registry::INFO,
				closure_as_pinned_box!(|mut context| {
					let repository_id_string =
						context.get_param(request_keys::REPOSITORY_ID).unwrap();
					let repository_id = hex::decode(&repository_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
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
			EveMiddleware::CustomFunction(pin_fn!(get_docker_repository_info)),
		],
	);

	// Get repository image details
	app.get(
		"/:repositoryId/image/:digest",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::docker_registry::INFO,
				closure_as_pinned_box!(|mut context| {
					let repo_id_string =
						context.get_param(request_keys::REPOSITORY_ID).unwrap();
					let repository_id = hex::decode(&repo_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
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
			EveMiddleware::CustomFunction(pin_fn!(
				get_repository_image_details
			)),
		],
	);

	// Get repository tag details
	app.get(
		"/:repositoryId/tag",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::docker_registry::INFO,
				closure_as_pinned_box!(|mut context| {
					let repo_id_string =
						context.get_param(request_keys::REPOSITORY_ID).unwrap();
					let repository_id = hex::decode(&repo_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
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
			EveMiddleware::CustomFunction(pin_fn!(get_list_of_repository_tags)),
		],
	);

	// Get repository tag details
	app.get(
		"/:repositoryId/tag/:tag",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::docker_registry::INFO,
				closure_as_pinned_box!(|mut context| {
					let repo_id_string =
						context.get_param(request_keys::REPOSITORY_ID).unwrap();
					let repository_id = hex::decode(&repo_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
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
			EveMiddleware::CustomFunction(pin_fn!(get_repository_tag_details)),
		],
	);

	// Delete repository image
	app.delete(
		"/:repositoryId/image/:digest",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::docker_registry::INFO,
				closure_as_pinned_box!(|mut context| {
					let repo_id_string =
						context.get_param(request_keys::REPOSITORY_ID).unwrap();
					let repository_id = hex::decode(&repo_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
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
			EveMiddleware::CustomFunction(pin_fn!(
				delete_docker_repository_image
			)),
		],
	);

	// Delete repository
	app.delete(
		"/:repositoryId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::docker_registry::DELETE,
				closure_as_pinned_box!(|mut context| {
					let repo_id_string =
						context.get_param(request_keys::REPOSITORY_ID).unwrap();
					let repository_id = hex::decode(&repo_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
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
/// # Description
/// This middleware creates a new docker repository
/// required inputs:
/// auth token in the authorization headers
/// workspace id in url
/// ```
/// {
///    repository: ,
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false,
///    id:
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn create_docker_repository(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	// check if the token is valid
	let CreateDockerRepositoryRequest {
		repository,
		workspace_id: _,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let repository = repository.trim().to_lowercase();

	let workspace_id_string =
		context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = hex::decode(&workspace_id_string).unwrap();

	// check if repo name is valid
	let is_repo_name_valid = validator::is_docker_repo_name_valid(&repository);
	if !is_repo_name_valid {
		context.status(400).json(error!(INVALID_REPOSITORY_NAME));
		return Ok(context);
	}

	// check if repository already exists
	let check = db::get_docker_repository_by_name(
		context.get_database_connection(),
		&repository,
		&workspace_id,
	)
	.await?;

	if check.is_some() {
		return Err(Error::empty()
			.status(400)
			.body(error!(RESOURCE_EXISTS).to_string()));
	}

	// split the repo name in 2 halves, and validate workspace, and repo name
	let resource_id =
		db::generate_new_resource_id(context.get_database_connection()).await?;
	let resource_id = resource_id.as_bytes();

	// call function to add repo details to the table
	// `docker_registry_repository` add a new resource
	db::create_resource(
		context.get_database_connection(),
		resource_id,
		&repository,
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::DOCKER_REPOSITORY)
			.unwrap(),
		&workspace_id,
		get_current_time_millis(),
	)
	.await?;
	db::create_docker_repository(
		context.get_database_connection(),
		resource_id,
		&repository,
		&workspace_id,
	)
	.await?;

	context.success(CreateDockerRepositoryResponse {
		id: Uuid::from_slice(&*resource_id)?,
	});
	Ok(context)
}

/// # Description
/// This function is used to list the docker repositories registered under
/// workspace
/// required inputs:
/// auth token in the authorization headers
/// workspace id in url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false,
///    repository:
///    [
///       {
///          id: ,
///          name:
///       }
///    ]
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn list_docker_repositories(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id_string =
		context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = hex::decode(&workspace_id_string).unwrap();

	let repositories = db::get_docker_repositories_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.into_iter()
	.filter_map(|(repository, size)| {
		Some(DockerRepository {
			id: Uuid::from_slice(&repository.id).ok()?,
			name: repository.name,
			size,
		})
	})
	.collect::<Vec<_>>();

	context.success(ListDockerRepositoriesResponse { repositories });
	Ok(context)
}

/// # Description
/// This function is used to get information about a docker repository
/// required inputs:
/// auth token in the authorization headers
/// repository id in url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_docker_repository_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let repository_id_string = context
		.get_param(request_keys::REPOSITORY_ID)
		.unwrap()
		.clone();
	let repository_id = hex::decode(&repository_id_string).unwrap();

	let repository = db::get_docker_repository_by_id(
		context.get_database_connection(),
		&repository_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let size = db::get_total_size_of_docker_repository(
		context.get_database_connection(),
		&repository_id,
	)
	.await?;
	let mut last_updated = 0; // TODO fetch this from the db for the sake of pagination

	let images = db::get_list_of_digests_for_docker_repository(
		context.get_database_connection(),
		&repository_id,
	)
	.await?;
	images.iter().for_each(|image| {
		last_updated = last_updated.max(image.created);
	});

	context.success(GetDockerRepositoryInfoResponse {
		repository: DockerRepository {
			id: Uuid::from_slice(&repository_id)?,
			name: repository.name,
			size,
		},
		images,
		last_updated,
	});
	Ok(context)
}

/// # Description
/// This function is used to get information about a docker repository's image
/// required inputs:
/// auth token in the authorization headers
/// repositoryId in the URL
/// image digest in the URL
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_repository_image_details(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let repository_id_string = context
		.get_param(request_keys::REPOSITORY_ID)
		.unwrap()
		.clone();
	let repository_id = hex::decode(&repository_id_string).unwrap();

	let digest = context.get_param(request_keys::DIGEST).unwrap().clone();

	let image = db::get_docker_repository_image_by_digest(
		context.get_database_connection(),
		&repository_id,
		&digest,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let tags = db::get_tags_for_docker_repository_image(
		context.get_database_connection(),
		&repository_id,
		&digest,
	)
	.await?;

	context.success(GetDockerRepositoryImageDetailsResponse { image, tags });
	Ok(context)
}

/// # Description
/// This function is used to get information about a docker repository's tag
/// required inputs:
/// auth token in the authorization headers
/// repositoryId in the URL
/// tag in the URL
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_list_of_repository_tags(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let repository_id_string = context
		.get_param(request_keys::REPOSITORY_ID)
		.unwrap()
		.clone();
	let repository_id = hex::decode(&repository_id_string).unwrap();

	let tags = db::get_list_of_tags_for_docker_repository(
		context.get_database_connection(),
		&repository_id,
	)
	.await?
	.into_iter()
	.map(|(tag_info, digest)| DockerRepositoryTagAndDigestInfo {
		tag_info,
		digest,
	})
	.collect();

	context.success(ListDockerRepositoryTagsResponse { tags });
	Ok(context)
}

/// # Description
/// This function is used to get information about a docker repository's tag
/// required inputs:
/// auth token in the authorization headers
/// repositoryId in the URL
/// tag in the URL
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_repository_tag_details(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let repository_id_string = context
		.get_param(request_keys::REPOSITORY_ID)
		.unwrap()
		.clone();
	let repository_id = hex::decode(&repository_id_string).unwrap();

	let tag = context.get_param(request_keys::TAG).unwrap().clone();

	let (tag_info, digest) = db::get_docker_repository_tag_details(
		context.get_database_connection(),
		&repository_id,
		&tag,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	context.success(GetDockerRepositoryTagDetailsResponse { tag_info, digest });
	Ok(context)
}

/// # Description
/// This function is used to delete a specific docker repository image inside a
/// repository.
/// required inputs:
/// auth token in the authorization headers
/// workspace id in url
/// ```
/// {
///    repositoryId:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn delete_docker_repository_image(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let repository_id_string = context
		.get_param(request_keys::REPOSITORY_ID)
		.unwrap()
		.clone();
	let repository_id = hex::decode(&repository_id_string).unwrap();
	let digest = context.get_param(request_keys::DIGEST).unwrap().clone();
	let config = context.get_state().config.clone();

	service::delete_docker_repository_image(
		context.get_database_connection(),
		&repository_id,
		&digest,
		&config,
	)
	.await?;

	context.success(DeleteDockerRepositoryImageResponse {});
	Ok(context)
}

/// # Description
/// This function is used to delete the docker repository present under the
/// workspace
/// required inputs:
/// auth token in the authorization headers
/// workspace id in url
/// ```
/// {
///    repositoryId:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn delete_docker_repository(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let repo_id_string =
		context.get_param(request_keys::REPOSITORY_ID).unwrap();
	let repository_id = hex::decode(&repo_id_string).unwrap();
	let config = context.get_state().config.clone();

	let running_deployments = db::get_deployments_by_repository_id(
		context.get_database_connection(),
		&repository_id,
	)
	.await?;
	if !running_deployments.is_empty() {
		Error::as_result()
			.status(400)
			.body(error!(RESOURCE_IN_USE).to_string())?;
	}

	// delete from docker registry using its API
	service::delete_docker_repository(
		context.get_database_connection(),
		&repository_id,
		&config,
	)
	.await?;

	context.success(DeleteDockerRepositoryResponse {});
	Ok(context)
}
