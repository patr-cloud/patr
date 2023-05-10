use api_models::{
	models::prelude::*,
	utils::{DtoRequestExt, Uuid},
};
use axum::{extract::State, Router};

use crate::{
	app::App,
	db::{self, ManagedUrlType as DbManagedUrlType},
	models::rbac::permissions,
	prelude::*,
	service,
	utils::Error,
};

pub fn create_sub_app(app: &App) -> Router<App> {
	Router::new()
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::managed_url::LIST,
				|ListManagedUrlsPath { workspace_id }, (), app, request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			),
			app.clone(),
			list_all_managed_urls,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::managed_url::EDIT,
				|CreateNewManagedUrlPath { workspace_id }, (), app, request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			),
			app.clone(),
			create_managed_url,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::managed_url::EDIT,
				|VerifyManagedUrlConfigurationPath {
				     workspace_id,
				     managed_url_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &managed_url_id)
						.await
						.filter(|value| value.owner_id == workspace_id);
				},
			),
			app.clone(),
			verify_managed_url_configuration,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::managed_url::DELETE,
				|UpdateManagedUrlPath {
				     workspace_id,
				     managed_url_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &managed_url_id)
						.await?
						.filter(|value| value.owner_id == workspace_id);
				},
			),
			app.clone(),
			update_managed_url,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::managed_database::LIST,
				|DeleteManagedUrlPath {
				     workspace_id,
				     managed_url_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &managed_url_id)
						.await?
						.filter(|value| value.owner_id == workspace_id);
				},
			),
			app.clone(),
			delete_managed_url,
		)
}

async fn list_all_managed_urls(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: ListManagedUrlsPath { workspace_id },
		query: (),
		body: (),
	}: DecodedRequest<ListManagedUrlsRequest>,
) -> Result<ListManagedUrlsResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {} - Listing all managed URLs", request_id);

	let urls =
		db::get_all_managed_urls_in_workspace(&mut connection, &workspace_id)
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
							ManagedUrlType::ProxyUrl {
								url: url.url?,
								http_only: url.http_only?,
							}
						}
						DbManagedUrlType::Redirect => {
							ManagedUrlType::Redirect {
								url: url.url?,
								permanent_redirect: url.permanent_redirect?,
								http_only: url.http_only?,
							}
						}
					},
					is_configured: url.is_configured,
				})
			})
			.collect();

	log::trace!("request_id: {} - Returning managed URLs", request_id);
	Ok(ListManagedUrlsResponse { urls })
}

async fn create_managed_url(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: CreateNewManagedUrlPath { workspace_id },
		query: (),
		body:
			CreateNewManagedUrlRequest {
				sub_domain,
				domain_id,
				path,
				url_type,
			},
	}: DecodedRequest<CreateNewManagedUrlRequest>,
) -> Result<CreateNewManagedUrlResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!(
		"{} - Creating new managed URL for workspace {}",
		request_id,
		workspace_id,
	);
	let id = service::create_new_managed_url_in_workspace(
		&mut connection,
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
	Ok(CreateNewManagedUrlResponse { id })
}

async fn update_managed_url(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: UpdateManagedUrlPath {
			workspace_id,
			managed_url_id,
		},
		query: (),
		body: UpdateManagedUrlRequest { path, url_type },
	}: DecodedRequest<UpdateManagedUrlRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();

	log::trace!(
		"request_id: {} - Updating managed URL {}",
		request_id,
		managed_url_id
	);

	service::update_managed_url(
		&mut connection,
		&managed_url_id,
		&path,
		&url_type,
		&config,
		&request_id,
	)
	.await?;

	Ok(())
}

async fn verify_managed_url_configuration(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path:
			VerifyManagedUrlConfigurationPath {
				workspace_id,
				managed_url_id,
			},
		query: (),
		body: (),
	}: DecodedRequest<VerifyManagedUrlConfigurationRequest>,
) -> Result<VerifyManagedUrlConfigurationResponse, Error> {
	let request_id = Uuid::new_v4();

	let configured = service::verify_managed_url_configuration(
		&mut connection,
		&managed_url_id,
		&config,
		&request_id,
	)
	.await?;

	db::update_managed_url_configuration_status(
		&mut connection,
		&managed_url_id,
		configured,
	)
	.await?;

	Ok(VerifyManagedUrlConfigurationResponse { configured });
}

async fn delete_managed_url(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: DeleteManagedUrlPath {
			workspace_id,
			managed_url_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<DeleteManagedUrlRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();

	db::get_managed_url_by_id(&mut connection, &managed_url_id)
		.await?
		.ok_or_else(|| ErrorType::NotFound)?;

	log::trace!(
		"request_id: {} - Deleting managed URL {}",
		request_id,
		managed_url_id
	);
	service::delete_managed_url(
		&mut connection,
		&workspace_id,
		&managed_url_id,
		&config,
		&request_id,
	)
	.await?;

	Ok(())
}
