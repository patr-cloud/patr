use api_macros::closure_as_pinned_box;
use api_models::{
	models::workspace::docker_registry::{
		CreateDockerRepositoryRequest,
		CreateDockerRepositoryResponse,
		DeleteDockerRepositoryImageResponse,
		DeleteDockerRepositoryResponse,
		DockerRepository,
		DockerRepositoryTagAndDigestInfo,
		GetDockerRepositoryExposedPortRequest,
		GetDockerRepositoryExposedPortResponse,
		GetDockerRepositoryImageDetailsResponse,
		GetDockerRepositoryInfoResponse,
		GetDockerRepositoryTagDetailsResponse,
		ListDockerRepositoriesResponse,
		ListDockerRepositoryTagsResponse,
	},
	utils::{DateTime, Uuid},
};
use axum::{
	routing::{delete, get, post},
	Router,
};
use chrono::Utc;
use eve_rs::{App as EveApp, AsError, Context, NextHandler};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::{
		rbac::{self, permissions},
		ResourceType,
	},
	pin_fn,
	service,
	utils::{
		constants::request_keys,
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
pub fn create_sub_route(app: &App) -> Router {
	let router = Router::new();

	// All routes have ResourceTokenAuthenticator middleware

	// create new repository
	router.route("/", post(create_docker_repository));

	// Get list of repositories
	router.route("/", get(list_docker_repositories));

	// Get repository info
	router.route("/:repositoryId", get(get_docker_repository_info));

	// Get exposed port
	router.route(
		"/:repositoryId/exposed-ports",
		get(get_repository_image_exposed_port),
	);

	// Get repository image details
	router.route(
		"/:repositoryId/image/:digest",
		get(get_repository_image_details),
	);

	// Get repository tag details
	router.route("/:repositoryId/tag", get(get_list_of_repository_tags));

	// Get repository tag details
	router.route("/:repositoryId/tag/:tag", get(get_repository_tag_details));

	// Delete repository image
	router.route(
		"/:repositoryId/image/:digest",
		delete(delete_docker_repository_image),
	);

	// Delete repository
	router.route("/:repositoryId", delete(delete_docker_repository));

	router
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
	let request_id = Uuid::new_v4();
	log::trace!(
		"request_id: {} - Creating docker repository in the workspace",
		request_id
	);
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
	let workspace_id = Uuid::parse_str(workspace_id_string).unwrap();

	// check if repo name is valid
	let is_repo_name_valid = validator::is_docker_repo_name_valid(&repository);
	if !is_repo_name_valid {
		context.status(400).json(error!(INVALID_REPOSITORY_NAME));
		return Ok(context);
	}

	// check if repository already exists
	log::trace!(
		"request_id: {} - Checking if repository already exists",
		request_id
	);
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

	// call function to add repo details to the table
	// `docker_registry_repository` add a new resource
	log::trace!(
		"request_id: {} - Creating a new resource in the database",
		request_id
	);
	db::create_resource(
		context.get_database_connection(),
		&resource_id,
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::DOCKER_REPOSITORY)
			.unwrap(),
		&workspace_id,
		&Utc::now(),
	)
	.await?;

	log::trace!(
		"request_id: {} - Adding a new docker repository in the database",
		request_id
	);
	db::create_docker_repository(
		context.get_database_connection(),
		&resource_id,
		&repository,
		&workspace_id,
	)
	.await?;

	log::trace!("request_id: {} - Docker repository created", request_id);
	context.success(CreateDockerRepositoryResponse { id: resource_id });
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
	let request_id = Uuid::new_v4();
	let workspace_id_string =
		context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id_string).unwrap();

	log::trace!("request_id: {} - Listing docker repositories", request_id);

	let repositories = db::get_docker_repositories_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.into_iter()
	.map(|(repository, size, last_updated)| DockerRepository {
		id: repository.id,
		name: repository.name,
		size,
		last_updated: DateTime(last_updated),
	})
	.collect::<Vec<_>>();

	log::trace!("request_id: {} - Docker repositories listed", request_id);

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
	let request_id = Uuid::new_v4();
	log::trace!(
		"request_id: {} - Getting docker repository info",
		request_id
	);
	let repository_id_string = context
		.get_param(request_keys::REPOSITORY_ID)
		.unwrap()
		.clone();
	let repository_id = Uuid::parse_str(&repository_id_string).unwrap();

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
	let last_updated = db::get_last_updated_for_docker_repository(
		context.get_database_connection(),
		&repository_id,
	)
	.await?;

	let images = db::get_list_of_digests_for_docker_repository(
		context.get_database_connection(),
		&repository_id,
	)
	.await?;

	log::trace!(
		"request_id: {} - Docker repository info fetched",
		request_id
	);

	context.success(GetDockerRepositoryInfoResponse {
		repository: DockerRepository {
			id: repository_id,
			name: repository.name,
			size,
			last_updated: DateTime(last_updated),
		},
		images,
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
	let request_id = Uuid::new_v4();
	log::trace!(
		"request_id: {} - Getting docker repository image details",
		request_id
	);
	let repository_id_string = context
		.get_param(request_keys::REPOSITORY_ID)
		.unwrap()
		.clone();
	let repository_id = Uuid::parse_str(&repository_id_string).unwrap();

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

	log::trace!(
		"request_id: {} - Docker repository image details fetched",
		request_id
	);
	context.success(GetDockerRepositoryImageDetailsResponse { image, tags });
	Ok(context)
}

async fn get_repository_image_exposed_port(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let config = context.get_state().config.to_owned();
	let repository_id = Uuid::parse_str(
		context.get_param(request_keys::REPOSITORY_ID).unwrap(),
	)
	.unwrap();

	let GetDockerRepositoryExposedPortRequest { tag, .. } = context
		.get_query_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let repository = db::get_docker_repository_by_id(
		context.get_database_connection(),
		&repository_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	db::get_docker_repository_tag_details(
		context.get_database_connection(),
		&repository_id,
		&tag,
	)
	.await?
	.status(404)
	.body(error!(TAG_NOT_FOUND).to_string())?;

	let repo_name = format!("{}/{}", repository.workspace_id, repository.name);

	let ports = service::get_exposed_port_for_docker_image(
		context.get_database_connection(),
		&config,
		&repo_name,
		&tag,
	)
	.await?;

	context.success(GetDockerRepositoryExposedPortResponse { ports });
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
	let request_id = Uuid::new_v4();
	log::trace!(
		"request_id: {} - Getting docker repository tags",
		request_id
	);
	let repository_id_string = context
		.get_param(request_keys::REPOSITORY_ID)
		.unwrap()
		.clone();
	let repository_id = Uuid::parse_str(&repository_id_string).unwrap();

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

	log::trace!(
		"request_id: {} - Docker repository tags fetched",
		request_id
	);
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
	let request_id = Uuid::new_v4();
	log::trace!(
		"request_id: {} - Getting docker repository tag details",
		request_id
	);
	let repository_id_string = context
		.get_param(request_keys::REPOSITORY_ID)
		.unwrap()
		.clone();
	let repository_id = Uuid::parse_str(&repository_id_string).unwrap();

	let tag = context.get_param(request_keys::TAG).unwrap().clone();

	let (tag_info, digest) = db::get_docker_repository_tag_details(
		context.get_database_connection(),
		&repository_id,
		&tag,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	log::trace!(
		"request_id: {} - Docker repository tag details fetched",
		request_id
	);
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
	let request_id = Uuid::new_v4();
	log::trace!(
		"request_id: {} - Deleting docker repository image",
		request_id
	);
	let repository_id_string = context
		.get_param(request_keys::REPOSITORY_ID)
		.unwrap()
		.clone();
	let repository_id = Uuid::parse_str(&repository_id_string).unwrap();
	let digest = context.get_param(request_keys::DIGEST).unwrap().clone();
	let config = context.get_state().config.clone();

	service::delete_docker_repository_image(
		context.get_database_connection(),
		&repository_id,
		&digest,
		&config,
		&request_id,
	)
	.await?;

	log::trace!(
		"request_id: {} - Docker repository image deleted",
		request_id
	);
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
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Deleting docker repository", request_id);
	let repo_id_string =
		context.get_param(request_keys::REPOSITORY_ID).unwrap();
	let repository_id = Uuid::parse_str(repo_id_string).unwrap();
	let config = context.get_state().config.clone();

	let user_id = context.get_token_data().unwrap().user_id().clone();

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

	// This is use after the deletion for sending mails
	let repository = db::get_docker_repository_by_id(
		context.get_database_connection(),
		&repository_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	// delete from docker registry using its API
	service::delete_docker_repository(
		context.get_database_connection(),
		&repository_id,
		&config,
		&request_id,
	)
	.await?;

	// Commiting transaction so that even if the mailing function fails the
	// resource should be deleted
	context.commit_database_transaction().await?;

	service::resource_delete_action_email(
		context.get_database_connection(),
		&repository.name,
		&repository.workspace_id,
		&ResourceType::DockerRepository,
		&user_id,
	)
	.await?;

	log::trace!("request_id: {} - Docker repository deleted", request_id);
	context.success(DeleteDockerRepositoryResponse {});
	Ok(context)
}
