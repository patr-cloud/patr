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
		VerifyManagedUrlConfigurationResponse,
	},
	utils::Uuid,
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};

use crate::{
	app::{create_eve_app, App},
	db::{self, ManagedUrlType as DbManagedUrlType},
	error,
	models::rbac::permissions,
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
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::managed_url::LIST,
				resource: closure_as_pinned_box!(|mut context| {
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
			},
			EveMiddleware::CustomFunction(pin_fn!(list_all_managed_urls)),
		],
	);

	// Create a new managed URL
	app.post(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::managed_url::CREATE,
				resource: closure_as_pinned_box!(|mut context| {
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
			},
			EveMiddleware::CustomFunction(pin_fn!(create_managed_url)),
		],
	);

	// Verify configuration of a managed URL
	app.post(
		"/:managedUrlId/verify-configuration",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::managed_url::EDIT,
				resource: closure_as_pinned_box!(|mut context| {
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
			},
			EveMiddleware::CustomFunction(pin_fn!(
				verify_managed_url_configuration
			)),
		],
	);

	// Verify configuration of a managed URL
	app.get(
		"/:managedUrlId/verification/:token",
		[EveMiddleware::CustomFunction(pin_fn!(
			get_managed_url_authorization_header
		))],
	);

	// Update a managed URL
	app.post(
		"/:managedUrlId",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::managed_url::EDIT,
				resource: closure_as_pinned_box!(|mut context| {
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
			},
			EveMiddleware::CustomFunction(pin_fn!(update_managed_url)),
		],
	);

	// Delete a managed URL
	app.delete(
		"/:managedUrlId",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::managed_url::DELETE,
				resource: closure_as_pinned_box!(|mut context| {
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
			},
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
	let urls = db::get_all_managed_urls_in_workspace(
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
			is_configured: url.is_configured,
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

async fn verify_managed_url_configuration(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	let managed_url_id = Uuid::parse_str(
		context.get_param(request_keys::MANAGED_URL_ID).unwrap(),
	)
	.unwrap();

	let config = context.get_state().config.clone();

	let configured = service::verify_managed_url_configuration(
		context.get_database_connection(),
		&managed_url_id,
		&config,
		&request_id,
	)
	.await?;

	if configured {
		db::update_managed_url_configuration_status(
			context.get_database_connection(),
			&managed_url_id,
			true,
		)
		.await?;
	}

	context.success(VerifyManagedUrlConfigurationResponse { configured });
	Ok(context)
}

async fn get_managed_url_authorization_header(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let verification_token =
		context.get_param(request_keys::TOKEN).unwrap().clone();
	context.body(&verification_token);
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

	db::get_managed_url_by_id(
		context.get_database_connection(),
		&managed_url_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

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
