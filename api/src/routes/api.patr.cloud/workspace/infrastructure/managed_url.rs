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
	db,
	error,
	models::{
		db_mapping::ManagedUrlType as DbManagedUrlType,
		rbac::permissions,
	},
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
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::deployment::managed_url::LIST,
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
			EveMiddleware::CustomFunction(pin_fn!(list_all_managed_urls)),
		],
	);

	// Create a new managed URL
	app.post(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::deployment::managed_url::CREATE,
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
				permissions::workspace::deployment::managed_url::EDIT,
				closure_as_pinned_box!(|mut context| {
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
					.await?;

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
				permissions::workspace::deployment::managed_url::DELETE,
				closure_as_pinned_box!(|mut context| {
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
					.await?;

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
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let urls = db::get_all_managed_urls_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
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

	context.success(ListManagedUrlsResponse { urls });
	Ok(context)
}

async fn create_managed_url(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
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

	let id = service::create_new_managed_url_in_workspace(
		context.get_database_connection(),
		&workspace_id,
		&sub_domain,
		&domain_id,
		&path,
		&url_type,
		&config,
	)
	.await?;

	context.success(CreateNewManagedUrlResponse { id });
	Ok(context)
}

async fn update_managed_url(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let managed_url_id = Uuid::parse_str(
		context.get_param(request_keys::MANAGED_URL_ID).unwrap(),
	)
	.unwrap();

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
	)
	.await?;

	context.success(UpdateManagedUrlResponse {});
	Ok(context)
}

async fn delete_managed_url(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let managed_url_id = Uuid::parse_str(
		context.get_param(request_keys::MANAGED_URL_ID).unwrap(),
	)
	.unwrap();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let config = context.get_state().config.clone();

	service::delete_managed_url(
		context.get_database_connection(),
		&workspace_id,
		&managed_url_id,
		&config,
	)
	.await?;

	context.success(DeleteManagedUrlResponse {});
	Ok(context)
}
