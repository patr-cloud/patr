use api_macros::closure_as_pinned_box;
use api_models::{
	models::workspace::project::{
		CreateProjectRequest,
		CreateProjectResponse,
		DeleteProjectResponse,
		GetProjectInfoResponse,
		ListProjectsResponse,
		Project,
		UpdateProjectRequest,
		UpdateProjectResponse,
	},
	utils::Uuid,
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac::{self, permissions},
	pin_fn,
	utils::{
		constants::request_keys,
		get_current_time_millis,
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

	// List all projects
	app.get(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::project::LIST,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
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
			EveMiddleware::CustomFunction(pin_fn!(list_projects)),
		],
	);

	// Create a new project
	app.post(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::project::CREATE,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
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
			EveMiddleware::CustomFunction(pin_fn!(create_project)),
		],
	);

	// get a project info
	app.get(
		"/:projectId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::project::INFO,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let project_id =
						context.get_param(request_keys::PROJECT_ID).unwrap();
					let project_id = Uuid::parse_str(project_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&project_id,
					)
					.await?
					.filter(|resource| resource.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(get_project_info)),
		],
	);

	// update project
	app.patch(
		"/:projectId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::project::EDIT,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let project_id =
						context.get_param(request_keys::PROJECT_ID).unwrap();
					let project_id = Uuid::parse_str(project_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&project_id,
					)
					.await?
					.filter(|resource| resource.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(update_project)),
		],
	);

	// delete project
	app.delete(
		"/:projectId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::project::DELETE,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let project_id =
						context.get_param(request_keys::PROJECT_ID).unwrap();
					let project_id = Uuid::parse_str(project_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&project_id,
					)
					.await?
					.filter(|resource| resource.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(delete_project)),
		],
	);

	app
}

async fn list_projects(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Listing all projects", request_id);

	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let projects = db::get_all_projects_in_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.into_iter()
	.map(|project| Project {
		id: project.id,
		name: project.name,
		description: project.description,
	})
	.collect();

	log::trace!("request_id: {} - Returning projects", request_id);
	context.success(ListProjectsResponse { projects });
	Ok(context)
}

async fn get_project_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - get project info", request_id);

	let project_id =
		Uuid::parse_str(context.get_param(request_keys::PROJECT_ID).unwrap())
			.unwrap();

	let project =
		db::get_project_by_id(context.get_database_connection(), &project_id)
			.await?
			.map(|project| Project {
				id: project.id,
				name: project.name,
				description: project.description,
			})
			.status(404)
			.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	log::trace!("request_id: {} - returning project info", request_id);
	context.success(GetProjectInfoResponse { project });
	Ok(context)
}

async fn create_project(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	log::trace!("{} - Creating new project", request_id);

	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let CreateProjectRequest {
		name, description, ..
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let resource_id =
		db::generate_new_resource_id(context.get_database_connection()).await?;

	db::create_resource(
		context.get_database_connection(),
		&resource_id,
		&name,
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::PROJECT)
			.unwrap(),
		&workspace_id,
		get_current_time_millis(),
	)
	.await?;

	db::create_project(
		context.get_database_connection(),
		&resource_id,
		&workspace_id,
		&name,
		&description,
	)
	.await?;

	log::trace!("request_id: {} - Returning new project", request_id);
	context.success(CreateProjectResponse { id: resource_id });
	Ok(context)
}

async fn update_project(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	let project_id =
		Uuid::parse_str(context.get_param(request_keys::PROJECT_ID).unwrap())
			.unwrap();

	log::trace!(
		"request_id: {} - Deleting secret {}",
		request_id,
		project_id
	);

	let UpdateProjectRequest {
		name, description, ..
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	db::update_project(
		context.get_database_connection(),
		&project_id,
		name.as_deref(),
		description.as_deref(),
	)
	.await?;

	context.success(UpdateProjectResponse {});
	Ok(context)
}

async fn delete_project(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	let project_id =
		Uuid::parse_str(context.get_param(request_keys::PROJECT_ID).unwrap())
			.unwrap();
	log::trace!(
		"request_id: {} - Deleting project {}",
		request_id,
		project_id
	);

	// TODO: validate whether other resources associated with project are
	// deleted
	db::delete_project(context.get_database_connection(), &project_id).await?;

	log::trace!(
		"request_id: {} - Deleted project {}",
		request_id,
		project_id
	);
	context.success(DeleteProjectResponse {});
	Ok(context)
}
