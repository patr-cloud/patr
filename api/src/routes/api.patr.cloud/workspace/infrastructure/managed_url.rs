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
use axum::{
	routing::{delete, get, post},
	Router,
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

pub fn create_sub_route(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let router = Router::new();

	//  All route have ResourceTokenAuthenticator middleware

	// List all managed URLs
	router.route("/", get(list_all_managed_urls));

	// Create a new managed URL
	router.route("/", post(create_managed_url));

	// Verify configuration of a managed URL
	router.route(
		"/:managedUrlId/verify-configuration",
		post(verify_managed_url_configuration),
	);

	// Update a managed URL
	router.route("/:managedUrlId", post(update_managed_url));

	// Delete a managed URL
	router.route("/:managedUrlId", delete(delete_managed_url));

	router
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
				DbManagedUrlType::ProxyUrl => ManagedUrlType::ProxyUrl {
					url: url.url?,
					http_only: url.http_only?,
				},
				DbManagedUrlType::Redirect => ManagedUrlType::Redirect {
					url: url.url?,
					permanent_redirect: url.permanent_redirect?,
					http_only: url.http_only?,
				},
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

	db::update_managed_url_configuration_status(
		context.get_database_connection(),
		&managed_url_id,
		configured,
	)
	.await?;

	context.success(VerifyManagedUrlConfigurationResponse { configured });
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
