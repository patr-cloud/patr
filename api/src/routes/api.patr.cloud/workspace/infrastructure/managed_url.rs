use api_macros::closure_as_pinned_box;
use api_models::{
	models::workspace::infrastructure::managed_urls::{
		CreateNewManagedUrlRequest,
		CreateNewManagedUrlResponse,
		DeleteManagedUrlResponse,
		ListManagedUrlsResponse,
		ManagedUrl,
		ManagedUrlType,
		UpdateManagedUrlRequest,
		UpdateManagedUrlResponse,
	},
	utils::Uuid,
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};

use crate::{
	app::{create_eve_app, App},
	db::{self, ManagedUrlType as DbManagedUrlType},
	error,
	models::rbac::{self, permissions},
	pin_fn,
	service,
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

	// List all managed URLs
	app.get(
		"/",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(list_all_managed_urls)),
		],
	);

	// Create a new managed URL
	app.post(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::managed_url::CREATE,
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
			EveMiddleware::CustomFunction(pin_fn!(create_managed_url)),
		],
	);

	// Update a managed URL
	app.post(
		"/:managedUrlId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::managed_url::EDIT,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let managed_url_id = context
						.get_param(request_keys::MANAGED_URL_ID)
						.unwrap();
					let managed_url_id = Uuid::parse_str(managed_url_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&managed_url_id,
					)
					.await?
					.filter(|value| value.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(update_managed_url)),
		],
	);

	// Delete a managed URL
	app.delete(
		"/:managedUrlId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::managed_url::DELETE,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let managed_url_id = context
						.get_param(request_keys::MANAGED_URL_ID)
						.unwrap();
					let managed_url_id = Uuid::parse_str(managed_url_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&managed_url_id,
					)
					.await?
					.filter(|value| value.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(delete_managed_url)),
		],
	);

	app
}

async fn list_all_managed_urls(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {} - Listing all managed URLs", request_id);
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let permission_id = rbac::PERMISSIONS
		.get()
		.unwrap()
		.get(permissions::workspace::infrastructure::managed_url::INFO)
		.unwrap();

	if !context
		.get_token_data()
		.unwrap()
		.workspaces
		.contains_key(&workspace_id)
	{
		Error::as_result()
			.status(404)
			.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;
	}

	log::trace!("request_id: {} - Getting the list of all managed urls for the workspace", request_id);
	let urls = db::get_all_managed_urls_in_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?;

	let workspace_permission = context
		.get_token_data()
		.unwrap()
		.workspaces
		.get(&workspace_id)
		.unwrap();

	let is_super_admin = workspace_permission.is_super_admin;
	if is_super_admin {
		let urls = urls
			.into_iter()
			.filter_map(|url| {
				Some(ManagedUrl {
					id: url.id,
					sub_domain: url.sub_domain,
					domain_id: url.domain_id,
					path: url.path,
					url_type: match url.url_type {
						DbManagedUrlType::ProxyToDeployment => {
							ManagedUrlType::ProxyDeployment {
								deployment_id: url.deployment_id?,
								port: url.port? as u16,
							}
						}
						DbManagedUrlType::ProxyToStaticSite => {
							ManagedUrlType::ProxyStaticSite {
								static_site_id: url.static_site_id?,
							}
						}
						DbManagedUrlType::ProxyUrl => {
							ManagedUrlType::ProxyUrl { url: url.url? }
						}
						DbManagedUrlType::Redirect => {
							ManagedUrlType::Redirect { url: url.url? }
						}
					},
				})
			})
			.collect();
		log::trace!("request_id: {} - Returning managed URLs", request_id);
		context.success(ListManagedUrlsResponse { urls });
		return Ok(context);
	}

	let resources = workspace_permission.resources.clone();
	let mut permitted_urls = Vec::new();
	for url in urls {
		if resources
			.get(&url.id)
			.map_or(false, |permissions| permissions.contains(permission_id))
		{
			permitted_urls.push(url);
		}
	}

	if permitted_urls.is_empty() {
		Error::as_result()
			.status(404)
			.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;
	}
	let urls = permitted_urls
		.into_iter()
		.filter_map(|url| {
			Some(ManagedUrl {
				id: url.id,
				sub_domain: url.sub_domain,
				domain_id: url.domain_id,
				path: url.path,
				url_type: match url.url_type {
					DbManagedUrlType::ProxyToDeployment => {
						ManagedUrlType::ProxyDeployment {
							deployment_id: url.deployment_id?,
							port: url.port? as u16,
						}
					}
					DbManagedUrlType::ProxyToStaticSite => {
						ManagedUrlType::ProxyStaticSite {
							static_site_id: url.static_site_id?,
						}
					}
					DbManagedUrlType::ProxyUrl => {
						ManagedUrlType::ProxyUrl { url: url.url? }
					}
					DbManagedUrlType::Redirect => {
						ManagedUrlType::Redirect { url: url.url? }
					}
				},
			})
		})
		.collect();

	log::trace!("request_id: {} - Returning managed URLs", request_id);
	context.success(ListManagedUrlsResponse { urls });
	Ok(context)
}

async fn create_managed_url(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let CreateNewManagedUrlRequest {
		workspace_id: _,
		sub_domain,
		domain_id,
		path,
		url_type,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let config = context.get_state().config.clone();

	log::trace!(
		"{} - Creating new managed URL for workspace {}",
		request_id,
		workspace_id,
	);
	let id = service::create_new_managed_url_in_workspace(
		context.get_database_connection(),
		&workspace_id,
		&sub_domain,
		&domain_id,
		&path,
		&url_type,
		&config,
		&request_id,
	)
	.await?;

	log::trace!("request_id: {} - Returning new managed URL", request_id);
	context.success(CreateNewManagedUrlResponse { id });
	Ok(context)
}

async fn update_managed_url(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let managed_url_id = Uuid::parse_str(
		context.get_param(request_keys::MANAGED_URL_ID).unwrap(),
	)
	.unwrap();

	log::trace!(
		"request_id: {} - Updating managed URL {}",
		request_id,
		managed_url_id
	);
	let UpdateManagedUrlRequest {
		managed_url_id: _,
		workspace_id: _,
		path,
		url_type,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let config = context.get_state().config.clone();

	service::update_managed_url(
		context.get_database_connection(),
		&managed_url_id,
		&path,
		&url_type,
		&config,
		&request_id,
	)
	.await?;

	context.success(UpdateManagedUrlResponse {});
	Ok(context)
}

async fn delete_managed_url(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	let managed_url_id = Uuid::parse_str(
		context.get_param(request_keys::MANAGED_URL_ID).unwrap(),
	)
	.unwrap();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let config = context.get_state().config.clone();

	log::trace!(
		"request_id: {} - Deleting managed URL {}",
		request_id,
		managed_url_id
	);
	service::delete_managed_url(
		context.get_database_connection(),
		&workspace_id,
		&managed_url_id,
		&config,
		&request_id,
	)
	.await?;

	context.success(DeleteManagedUrlResponse {});
	Ok(context)
}
