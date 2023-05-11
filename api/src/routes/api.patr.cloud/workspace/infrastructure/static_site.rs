use api_models::{
	models::prelude::{static_site::ListLinkedURLsPath, *},
	utils::{DateTime, Uuid},
};
use axum::{extract::State, Extension, Router};

use crate::{
	app::App,
	db::{self, ManagedUrlType as DbManagedUrlType},
	models::{rbac::permissions, ResourceType, UserAuthenticationData},
	prelude::*,
	service,
	utils::Error,
};

pub fn create_sub_app(app: &App) -> Router<App> {
	Router::new()
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::static_site::LIST,
				|ListStaticSitesPath { workspace_id }, (), app, request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			),
			app.clone(),
			list_static_sites,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::static_site::LIST,
				|ListStaticSiteUploadHistoryPath {
				     workspace_id,
				     static_site_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &static_site_id)
						.await?
						.map(|resource| resource.owner_id == workspace_id)
				},
			),
			app.clone(),
			list_static_sites_upload_history,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::static_site::INFO,
				|GetStaticSiteInfoPath {
				     workspace_id,
				     static_site_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &static_site_id)
						.await?
						.map(|resource| resource.owner_id == workspace_id)
				},
			),
			app.clone(),
			get_static_site_info,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::static_site::EDIT,
				|StartStaticSitePath {
				     workspace_id,
				     static_site_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &static_site_id)
						.await?
						.map(|resource| resource.owner_id == workspace_id)
				},
			),
			app.clone(),
			start_static_site,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::static_site::EDIT,
				|UpdateStaticSitePath {
				     workspace_id,
				     static_site_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &static_site_id)
						.await?
						.map(|resource| resource.owner_id == workspace_id)
				},
			),
			app.clone(),
			update_static_site,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::static_site::EDIT,
				|UploadStaticSitePath {
				     workspace_id,
				     static_site_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &static_site_id)
						.await?
						.map(|resource| resource.owner_id == workspace_id)
				},
			),
			app.clone(),
			upload_static_site,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::static_site::EDIT,
				|RevertStaticSitePath {
				     workspace_id,
				     static_site_id,
				     upload_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &static_site_id)
						.await?
						.map(|resource| resource.owner_id == workspace_id)
				},
			),
			app.clone(),
			revert_static_site,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::static_site::EDIT,
				|StopStaticSitePath {
				     workspace_id,
				     static_site_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &static_site_id)
						.await?
						.map(|resource| resource.owner_id == workspace_id)
				},
			),
			app.clone(),
			stop_static_site,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::static_site::CREATE,
				|CreateStaticSitePath { workspace_id }, (), app, request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			),
			app.clone(),
			create_static_site,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::static_site::DELETE,
				|DeleteStaticSitePath {
				     workspace_id,
				     static_site_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &static_site_id)
						.await?
						.map(|resource| resource.owner_id == workspace_id)
				},
			),
			app.clone(),
			delete_static_site,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::static_site::LIST,
				|ListLinkedURLsPath {
				     workspace_id,
				     static_site_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &static_site_id)
						.await?
						.map(|resource| resource.owner_id == workspace_id)
				},
			),
			app.clone(),
			list_linked_urls,
		)
}

async fn get_static_site_info(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: GetStaticSiteInfoPath {
			workspace_id,
			static_site_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<GetStaticSiteInfoRequest>,
) -> Result<GetStaticSiteInfoResponse, Error> {
	let static_site =
		db::get_static_site_by_id(&mut connection, &static_site_id)
			.await?
			.ok_or_else(|| ErrorType::NotFound)?;

	Ok(GetStaticSiteInfoResponse {
		static_site: StaticSite {
			id: static_site.id,
			name: static_site.name,
			status: static_site.status,
			current_live_upload: static_site.current_live_upload,
		},
		static_site_details: StaticSiteDetails {},
	})
}

async fn list_static_sites(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: ListStaticSitesPath { workspace_id },
		query: (),
		body: (),
	}: DecodedRequest<ListStaticSitesRequest>,
) -> Result<ListStaticSitesResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {} - Getting the list of all static sites for the workspace", request_id);
	let static_sites =
		db::get_static_sites_for_workspace(&mut connection, &workspace_id)
			.await?
			.into_iter()
			.map(|static_site| StaticSite {
				id: static_site.id,
				name: static_site.name,
				status: static_site.status,
				current_live_upload: static_site.current_live_upload,
			})
			.collect::<Vec<_>>();
	log::trace!("request_id: {} - Returning the list of all static sites for the workspace", request_id);

	Ok(ListStaticSitesResponse { static_sites })
}

async fn list_static_sites_upload_history(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path:
			ListStaticSiteUploadHistoryPath {
				workspace_id,
				static_site_id,
			},
		query: (),
		body: (),
	}: DecodedRequest<ListStaticSiteUploadHistoryRequest>,
) -> Result<ListStaticSiteUploadHistoryResponse, Error> {
	db::get_static_site_by_id(&mut connection, &static_site_id)
		.await?
		.ok_or_else(|| ErrorType::NotFound)?;

	let uploads =
		db::get_static_site_upload_history(&mut connection, &static_site_id)
			.await?
			.into_iter()
			.map(|deploy_history| StaticSiteUploadHistory {
				upload_id: deploy_history.id,
				message: deploy_history.message,
				uploaded_by: deploy_history.uploaded_by,
				created: DateTime(deploy_history.created),
				processed: deploy_history.processed.map(DateTime),
			})
			.collect();

	Ok(ListStaticSiteUploadHistoryResponse { uploads });
}

async fn create_static_site(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: CreateStaticSitePath { workspace_id },
		query: (),
		body:
			CreateStaticSiteRequest {
				name,
				message,
				file,
				static_site_details,
			},
	}: DecodedRequest<CreateStaticSiteRequest>,
) -> Result<CreateStaticSiteResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {} - Creating a static site", request_id);

	let user_id = token_data.user_id().clone();

	let id = service::create_static_site_in_workspace(
		&mut connection,
		&workspace_id,
		&name.trim(),
		file,
		&message,
		&user_id,
		&config,
		&request_id,
	)
	.await?;

	log::trace!("request_id: {} - Static-site created", request_id);

	service::get_internal_metrics(
		&mut connection,
		"A static site has been created",
	)
	.await;
	Ok(CreateStaticSiteResponse { id })
}

async fn revert_static_site(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path:
			RevertStaticSitePath {
				workspace_id,
				static_site_id,
				upload_id,
			},
		query: (),
		body: (),
	}: DecodedRequest<RevertStaticSiteRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();

	// check if upload_id is present in the deploy history
	db::get_static_site_upload_history_by_upload_id(
		&mut connection,
		&static_site_id,
		&upload_id,
	)
	.await?
	.ok_or_else(|| ErrorType::NotFound)?;

	log::trace!("request_id: {} - Reverting static site", request_id);

	db::get_static_site_by_id(&mut connection, &static_site_id)
		.await?
		.ok_or_else(|| ErrorType::NotFound)?;

	service::update_cloudflare_running_upload(
		&mut connection,
		&static_site_id,
		&upload_id,
		&config,
		&request_id,
	)
	.await?;

	connection.commit().await?;

	Ok(());
}

async fn start_static_site(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: StartStaticSitePath {
			workspace_id,
			static_site_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<StartStaticSiteRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();

	log::trace!(
		"request_id: {} - Starting a static site with id: {}",
		request_id,
		static_site_id
	);

	// Get current_live_upload from static_site
	let static_site =
		db::get_static_site_by_id(&mut connection, &static_site_id)
			.await?
			.ok_or_else(|| ErrorType::NotFound)?;

	// Check if upload_id is present or not
	if let Some(upload_id) = static_site.current_live_upload {
		service::update_cloudflare_running_upload(
			&mut connection,
			&static_site_id,
			&upload_id,
			&config,
			&request_id,
		)
		.await?;
	} else {
		return Err(ErrorType::NotFound);
	};

	Ok(())
}

async fn update_static_site(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: UpdateStaticSitePath {
			workspace_id,
			static_site_id,
		},
		query: (),
		body: UpdateStaticSiteRequest { name },
	}: DecodedRequest<UpdateStaticSiteRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();

	log::trace!(
		"Updating static site with id: {} and request_id: {}",
		static_site_id,
		request_id
	);

	// Check if resource(static site exists)
	db::get_static_site_by_id(&mut connection, &static_site_id)
		.await?
		.ok_or_else(|| ErrorType::NotFound)?;

	db::update_static_site_name(&mut connection, &static_site_id, &name.trim())
		.await?;

	Ok(())
}

async fn upload_static_site(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: UploadStaticSitePath {
			workspace_id,
			static_site_id,
		},
		query: (),
		body: UploadStaticSiteRequest { message, file },
	}: DecodedRequest<UploadStaticSiteRequest>,
) -> Result<UploadStaticSiteResponse, Error> {
	let request_id = Uuid::new_v4();
	let user_id = token_data.user_id().clone();

	log::trace!(
		"Uploading the file for static site with id: {} and request_id: {}",
		static_site_id,
		request_id
	);

	// Check if resource(static site exists)
	db::get_static_site_by_id(&mut connection, &static_site_id)
		.await?
		.ok_or_else(|| ErrorType::NotFound)?;

	let upload_id = service::upload_static_site(
		&mut connection,
		&workspace_id,
		&static_site_id,
		file,
		&message,
		&user_id,
		&config,
		&request_id,
	)
	.await?;

	connection.commit().await?;

	log::trace!(
		"request_id: {} checking managed url for static_site with ID: {}",
		request_id,
		static_site_id
	);

	Ok(UploadStaticSiteResponse { upload_id })
}

async fn stop_static_site(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: StopStaticSitePath {
			workspace_id,
			static_site_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<StopStaticSiteRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	log::trace!(
		"request_id: {} - Stopping a static site with id: {}",
		request_id,
		static_site_id
	);

	// stop the running site, if it exists
	service::stop_static_site(
		&mut connection,
		&static_site_id,
		&config,
		&request_id,
	)
	.await?;

	Ok(())
}

async fn delete_static_site(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: DeleteStaticSitePath {
			workspace_id,
			static_site_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<DeleteStaticSiteRequest>,
) -> Result<(), Error> {
	let user_id = token_data.user_id().clone();

	let request_id = Uuid::new_v4();

	let site = db::get_static_site_by_id(&mut connection, &static_site_id)
		.await?
		.ok_or_else(|| ErrorType::NotFound)?;

	log::trace!("request_id: {} - Checking is any managed url is used by the deployment: {}", request_id, static_site_id);
	let managed_url =
		db::get_managed_urls_for_static_site(&mut connection, &static_site_id)
			.await?;

	if !managed_url.is_empty() {
		log::trace!(
			"static site: {} - is using managed_url. Cannot delete it",
			static_site_id
		);
		return Err(ErrorType::NotFound);
	}

	log::trace!(
		"request_id: {} - Deleting the static site with id: {}",
		request_id,
		static_site_id
	);

	// stop and delete the container running the image, if it exists
	service::delete_static_site(&mut connection, &static_site_id, &config)
		.await?;

	service::get_internal_metrics(
		&mut connection,
		"A static site has been deleted",
	)
	.await;

	// Commiting transaction so that even if the mailing function fails the
	// resource should be deleted
	connection.commit().await?;

	service::resource_delete_action_email(
		&mut connection,
		&site.name,
		&site.workspace_id,
		&ResourceType::StaticSite,
		&user_id,
	)
	.await?;

	Ok(())
}

async fn list_linked_urls(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: ListLinkedURLsPath {
			workspace_id,
			static_site_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<ListManagedUrlsRequest>,
) -> Result<ListManagedUrlsResponse, Error> {
	let request_id = Uuid::new_v4();
	log::trace!(
		"request_id: {} - Listing the linked urls for static site with id: {}",
		request_id,
		static_site_id
	);
	let urls = db::get_all_managed_urls_for_static_site(
		&mut connection,
		&static_site_id,
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

	Ok(())
}
