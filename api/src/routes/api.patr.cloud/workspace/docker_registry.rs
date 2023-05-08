use api_models::{
	models::prelude::*,
	utils::{DateTime, Paginated, Uuid},
};
use axum::{extract::State, Extension, Router};
use chrono::Utc;

use crate::{
	app::App,
	db,
	error,
	models::{
		rbac::{self, permissions},
		ResourceType,
		UserAuthenticationData,
	},
	prelude::*,
	service,
	utils::{validator, Error},
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
pub fn create_sub_app(app: &App) -> Router<App> {
	Router::new()
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::docker_registry::CREATE,
				|CreateDockerRepositoryPath { workspace_id },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			),
			app.clone(),
			create_docker_repository,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::docker_registry::LIST,
				|ListDockerRepositoriesPath { workspace_id },
				 Paginated {
				     start: _,
				     count: _,
				     query: (),
				 },
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			),
			app.clone(),
			list_docker_repositories,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::docker_registry::INFO,
				|GetDockerRepositoryInfoPath {
				     workspace_id,
				     repository_id,
				 },
				 Paginated {
				     start: _,
				     count: _,
				     query: (),
				 },
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &repository_id)
						.await
						.filter(|value| value.owner_id == workspace_id)
				},
			),
			app.clone(),
			get_docker_repository_info,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::docker_registry::INFO,
				|GetDockerRepositoryExposedPortPath {
				     workspace_id,
				     repository_id,
				 },
				 GetDockerRepositoryExposedPortRequest { tag },
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &repository_id)
						.await
						.filter(|value| value.owner_id == workspace_id)
				},
			),
			app.clone(),
			get_repository_image_exposed_port,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::docker_registry::INFO,
				|GetDockerRepositoryImageDetailsPath {
				     workspace_id,
				     repository_id,
				     image_digest,
				 },
				 Paginated {
				     start: _,
				     count: _,
				     query: (),
				 },
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &repository_id)
						.await
						.filter(|value| value.owner_id == workspace_id)
				},
			),
			app.clone(),
			get_repository_image_details,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::docker_registry::INFO,
				|ListDockerRepositoryTagsPath {
				     workspace_id,
				     repository_id,
				 },
				 Paginated {
				     start: _,
				     count: _,
				     query: (),
				 },
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &repository_id)
						.await
						.filter(|value| value.owner_id == workspace_id)
				},
			),
			app.clone(),
			get_list_of_repository_tags,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::docker_registry::INFO,
				|GetDockerRepositoryTagDetailsPath {
				     workspace_id,
				     repository_id,
				     tag,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &repository_id)
						.await
						.filter(|value| value.owner_id == workspace_id)
				},
			),
			app.clone(),
			get_repository_tag_details,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::docker_registry::DELETE,
				|DeleteDockerRepositoryImagePath {
				     workspace_id,
				     repository_id,
				     digest,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &repository_id)
						.await
						.filter(|value| value.owner_id == workspace_id)
				},
			),
			app.clone(),
			delete_docker_repository_image,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::docker_registry::DELETE,
				|DeleteDockerRepositoryPath {
				     workspace_id,
				     repository_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &repository_id)
						.await
						.filter(|value| value.owner_id == workspace_id)
				},
			),
			app.clone(),
			delete_docker_repository,
		)
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
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: CreateDockerRepositoryPath { workspace_id },
		query: (),
		body: CreateDockerRepositoryRequest { repository },
	}: DecodedRequest<CreateDockerRepositoryRequest>,
) -> Result<CreateDockerRepositoryResponse, Error> {
	let request_id = Uuid::new_v4();
	log::trace!(
		"request_id: {} - Creating docker repository in the workspace",
		request_id
	);
	// check if the token is valid
	let repository = repository.trim().to_lowercase();

	// check if repo name is valid
	let is_repo_name_valid = validator::is_docker_repo_name_valid(&repository);
	if !is_repo_name_valid {
		return Err(ErrorType::InvalidRepositoryName);
	}

	// check if repository already exists
	log::trace!(
		"request_id: {} - Checking if repository already exists",
		request_id
	);
	let check = db::get_docker_repository_by_name(
		&mut connection,
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
	let resource_id = db::generate_new_resource_id(&mut connection).await?;

	// call function to add repo details to the table
	// `docker_registry_repository` add a new resource
	log::trace!(
		"request_id: {} - Creating a new resource in the database",
		request_id
	);
	db::create_resource(
		&mut connection,
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
		&mut connection,
		&resource_id,
		&repository,
		&workspace_id,
	)
	.await?;

	log::trace!("request_id: {} - Docker repository created", request_id);
	Ok(CreateDockerRepositoryResponse { id: resource_id })
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
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: ListDockerRepositoriesPath { workspace_id },
		query: Paginated {
			start: _,
			count: _,
			query: (),
		},
		body: (),
	}: DecodedRequest<ListDockerRepositoriesRequest>,
) -> Result<ListDockerRepositoriesResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {} - Listing docker repositories", request_id);

	let repositories = db::get_docker_repositories_for_workspace(
		&mut connection,
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

	Ok(ListDockerRepositoriesResponse { repositories })
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
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path:
			GetDockerRepositoryInfoPath {
				workspace_id,
				repository_id,
			},
		query: Paginated {
			start: _,
			count: _,
			query: (),
		},
		body: (),
	}: DecodedRequest<GetDockerRepositoryInfoRequest>,
) -> Result<GetDockerRepositoryInfoResponse, Error> {
	let request_id = Uuid::new_v4();
	log::trace!(
		"request_id: {} - Getting docker repository info",
		request_id
	);

	let repository =
		db::get_docker_repository_by_id(&mut connection, &repository_id)
			.await?
			.ok_or_else(|| ErrorType::NotFound)?;

	let size = db::get_total_size_of_docker_repository(
		&mut connection,
		&repository_id,
	)
	.await?;
	let last_updated = db::get_last_updated_for_docker_repository(
		&mut connection,
		&repository_id,
	)
	.await?;

	let images = db::get_list_of_digests_for_docker_repository(
		&mut connection,
		&repository_id,
	)
	.await?;

	log::trace!(
		"request_id: {} - Docker repository info fetched",
		request_id
	);

	Ok(GetDockerRepositoryInfoResponse {
		repository: DockerRepository {
			id: repository_id,
			name: repository.name,
			size,
			last_updated: DateTime(last_updated),
		},
		images,
	})
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
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path:
			GetDockerRepositoryImageDetailsPath {
				workspace_id,
				repository_id,
				image_digest,
			},
		query: Paginated {
			start: _,
			count: _,
			query: (),
		},
		body: (),
	}: DecodedRequest<GetDockerRepositoryImageDetailsRequest>,
) -> Result<GetDockerRepositoryImageDetailsResponse, Error> {
	let request_id = Uuid::new_v4();
	log::trace!(
		"request_id: {} - Getting docker repository image details",
		request_id
	);

	let image = db::get_docker_repository_image_by_digest(
		&mut connection,
		&repository_id,
		&image_digest,
	)
	.await?
	.ok_or_else(|| ErrorType::NotFound)?;

	let tags = db::get_tags_for_docker_repository_image(
		&mut connection,
		&repository_id,
		&image_digest,
	)
	.await?;

	log::trace!(
		"request_id: {} - Docker repository image details fetched",
		request_id
	);
	Ok(GetDockerRepositoryImageDetailsResponse { image, tags })
}

async fn get_repository_image_exposed_port(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path:
			GetDockerRepositoryExposedPortPath {
				workspace_id,
				repository_id,
			},
		query: GetDockerRepositoryExposedPortRequest { tag },
		body: (),
	}: DecodedRequest<GetDockerRepositoryExposedPortRequest>,
) -> Result<GetDockerRepositoryExposedPortResponse, Error> {
	let repository =
		db::get_docker_repository_by_id(&mut connection, &repository_id)
			.await?
			.ok_or_else(|| ErrorType::NotFound)?;

	db::get_docker_repository_tag_details(
		&mut connection,
		&repository_id,
		&tag,
	)
	.await?
	.ok_or_else(|| ErrorType::NotFound)?;

	let repo_name = format!("{}/{}", repository.workspace_id, repository.name);

	let ports = service::get_exposed_port_for_docker_image(
		&mut connection,
		&config,
		&repo_name,
		&tag,
	)
	.await?;

	Ok(GetDockerRepositoryExposedPortResponse { ports })
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
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path:
			ListDockerRepositoryTagsPath {
				workspace_id,
				repository_id,
			},
		query: Paginated {
			start: _,
			count: _,
			query: (),
		},
		body: (),
	}: DecodedRequest<ListDockerRepositoryTagsRequest>,
) -> Result<ListDockerRepositoryTagsResponse, Error> {
	let request_id = Uuid::new_v4();
	log::trace!(
		"request_id: {} - Getting docker repository tags",
		request_id
	);

	let tags = db::get_list_of_tags_for_docker_repository(
		&mut connection,
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
	Ok(ListDockerRepositoryTagsResponse { tags })
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
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path:
			GetDockerRepositoryTagDetailsPath {
				workspace_id,
				repository_id,
				tag,
			},
		query: (),
		body: (),
	}: DecodedRequest<GetDockerRepositoryTagDetailsRequest>,
) -> Result<GetDockerRepositoryTagDetailsResponse, Error> {
	let request_id = Uuid::new_v4();
	log::trace!(
		"request_id: {} - Getting docker repository tag details",
		request_id
	);

	let (tag_info, digest) = db::get_docker_repository_tag_details(
		&mut connection,
		&repository_id,
		&tag,
	)
	.await?
	.ok_or_else(|| ErrorType::NotFound)?;

	log::trace!(
		"request_id: {} - Docker repository tag details fetched",
		request_id
	);
	Ok(GetDockerRepositoryTagDetailsResponse { tag_info, digest })
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
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path:
			DeleteDockerRepositoryImagePath {
				workspace_id,
				repository_id,
				digest,
			},
		query: (),
		body: (),
	}: DecodedRequest<DeleteDockerRepositoryImageRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	log::trace!(
		"request_id: {} - Deleting docker repository image",
		request_id
	);

	service::delete_docker_repository_image(
		&mut connection,
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
	Ok(())
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
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path:
			DeleteDockerRepositoryPath {
				workspace_id,
				repository_id,
			},
		query: (),
		body: (),
	}: DecodedRequest<DeleteDockerRepositoryRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Deleting docker repository", request_id);

	let running_deployments =
		db::get_deployments_by_repository_id(&mut connection, &repository_id)
			.await?;
	if !running_deployments.is_empty() {
		Error::as_result()
			.status(400)
			.body(error!(RESOURCE_IN_USE).to_string())?;
	}

	// This is use after the deletion for sending mails
	let repository =
		db::get_docker_repository_by_id(&mut connection, &repository_id)
			.await?
			.status(404)
			.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	// delete from docker registry using its API
	service::delete_docker_repository(
		&mut connection,
		&repository_id,
		&config,
		&request_id,
	)
	.await?;

	// Commiting transaction so that even if the mailing function fails the
	// resource should be deleted
	connection.commit().await?;

	service::resource_delete_action_email(
		&mut connection,
		&repository.name,
		&repository.workspace_id,
		&ResourceType::DockerRepository,
		&token_data.user_id(),
	)
	.await?;

	log::trace!("request_id: {} - Docker repository deleted", request_id);
	Ok(())
}
