use api_models::{
	models::prelude::*,
	utils::{DateTime, Paginated, Uuid},
};
use axum::{extract::State, Extension, Router};

use crate::{
	app::App,
	db::{self, DnsRecordType},
	models::{rbac::permissions, ResourceType, UserAuthenticationData},
	prelude::*,
	service,
	utils::Error,
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
				permissions::workspace::domain::LIST,
				|GetDomainsForWorkspacePath { workspace_id },
				 Paginated {
				     start,
				     count,
				     query: (),
				 },
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id)
						.await?;
				},
			),
			app.clone(),
			get_domains_for_workspace,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::domain::ADD,
				|AddDomainPath { workspace_id }, (), app, request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id)
						.await?;
				},
			),
			app.clone(),
			add_domain_to_workspace,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::domain::VERIFY,
				|VerifyDomainPath {
				     workspace_id,
				     domain_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &domain_id)
						.await?
						.filter(|value| value.owner_id == workspace_id);
				},
			),
			app.clone(),
			verify_domain_in_workspace,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::domain::VIEW_DETAILS,
				|GetDomainInfoPath {
				     workspace_id,
				     domain_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &domain_id)
						.await?
						.filter(|value| value.owner_id == workspace_id);
				},
			),
			app.clone(),
			get_domain_info_in_workspace,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::domain::DELETE,
				|DeleteDomainPath {
				     workspace_id,
				     domain_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &domain_id)
						.await?
						.filter(|value| value.owner_id == workspace_id);
				},
			),
			app.clone(),
			delete_domain_in_workspace,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::domain::dns_record::LIST,
				|GetDomainDnsRecordsPath {
				     workspace_id,
				     domain_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &domain_id)
						.await?
						.filter(|value| value.owner_id == workspace_id);
				},
			),
			app.clone(),
			get_domain_dns_record,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::domain::dns_record::ADD,
				|AddDnsRecordPath {
				     workspace_id,
				     domain_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &domain_id)
						.await?
						.filter(|value| value.owner_id == workspace_id);
				},
			),
			app.clone(),
			add_dns_record,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::domain::dns_record::EDIT,
				|UpdateDomainDnsRecordPath {
				     workspace_id,
				     domain_id,
				     record_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &record_id)
						.await?
						.filter(|value| value.owner_id == workspace_id);
				},
			),
			app.clone(),
			update_dns_record,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::domain::dns_record::DELETE,
				|DeleteDnsRecordPath {
				     workspace_id,
				     domain_id,
				     record_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &record_id)
						.await?
						.filter(|value| value.owner_id == workspace_id);
				},
			),
			app.clone(),
			delete_dns_record,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::domain::ADD,
				|IsDomainPersonalPath { workspace_id },
				 IsDomainPersonalRequest { domain },
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id)
						.await?;
				},
			),
			app.clone(),
			is_domain_personal,
		)
}

/// # Description
/// This function is used to get the list of domains present under the
/// workspace
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
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
///    domains:
///    [
///       {
///          id:
///          name:
///          verified:
///       }
///    ]
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_domains_for_workspace(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path:
			GetDomainsForWorkspacePath {
				workspace_id,
				domain_id,
				record_id,
			},
		query: Paginated {
			start,
			count,
			query: (),
		},
		body: (),
	}: DecodedRequest<GetDomainsForWorkspaceRequest>,
) -> Result<GetDomainsForWorkspaceResponse, Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Getting domains for workspace", request_id);

	let domains = db::get_domains_for_workspace(&mut connection, &workspace_id)
		.await?
		.into_iter()
		.map(|domain| WorkspaceDomain {
			domain: Domain {
				id: domain.id,
				name: domain.name,
				last_unverified: domain.last_unverified.map(DateTime),
			},
			is_verified: domain.is_verified,
			nameserver_type: domain.nameserver_type,
		})
		.collect();

	log::trace!(
		"request_id: {} - Returning domains for workspace",
		request_id
	);
	Ok(GetDomainsForWorkspaceResponse { domains })
}

/// # Description
/// This function is used to add a domain to an workspace
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// workspace id in url
/// ```
/// {
///     domain:
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
///    domainId:
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn add_domain_to_workspace(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: AddDomainPath { workspace_id },
		query: (),
		body: AddDomainRequest {
			domain,
			nameserver_type,
		},
	}: DecodedRequest<AddDomainRequest>,
) -> Result<AddDomainResponse, Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Adding domain to workspace", request_id);

	let domain_id = service::add_domain_to_workspace(
		&mut connection,
		&domain,
		&nameserver_type,
		&workspace_id,
		&config,
		&request_id,
	)
	.await?;

	log::trace!(
		"request_id: {} - Added the domain to the workspace",
		request_id
	);

	Ok(AddDomainResponse { id: domain_id })
}

/// # Description
/// This function is used to verify a domain which is to be registered under a
/// workspace
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// workspace id in the url
/// ```
/// {
///     domainId:
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
async fn verify_domain_in_workspace(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: VerifyDomainPath {
			workspace_id,
			domain_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<VerifyDomainRequest>,
) -> Result<VerifyDomainResponse, Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Verifying domain in workspace", request_id);

	let domain = db::get_workspace_domain_by_id(&mut connection, &domain_id)
		.await?
		.ok_or_else(|| ErrorType::internal_error())?;

	let verified = service::is_domain_verified(
		&mut connection,
		&domain.id,
		&workspace_id,
		&config,
		&request_id,
	)
	.await?;

	Ok(VerifyDomainResponse { verified })
}

/// # Description
/// This function is used to get details about a domain registered under the
/// workspace
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// workspace id in the url
/// ```
/// {
///     domainId:
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
///    domainId: ,
///    name: ,
///    verified: true
/// }
/// if not verified
/// {
///    success: true or false,
///    domainId: ,
///    name: ,
///    verified: true,
///    verificationToken:
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_domain_info_in_workspace(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: GetDomainInfoPath {
			workspace_id,
			domain_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<GetDomainInfoRequest>,
) -> Result<GetDomainInfoResponse, Error> {
	let request_id = Uuid::new_v4();
	log::trace!(
		"request_id: {} - Getting domain info in workspace",
		request_id
	);

	let domain = db::get_workspace_domain_by_id(&mut connection, &domain_id)
		.await?
		.ok_or_else(|| ErrorType::NotFound)?;

	log::trace!("request_id: {} - Got domain info in workspace", request_id);
	Ok(GetDomainInfoResponse {
		workspace_domain: WorkspaceDomain {
			domain: Domain {
				id: domain.id,
				name: domain.name,
				last_unverified: domain.last_unverified.map(DateTime),
			},
			is_verified: domain.is_verified,
			nameserver_type: domain.nameserver_type,
		},
	})
}

/// # Description
/// This function is used to delete the domain registered under the workspace
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// workspace id in the url
/// ```
/// {
///    domainId:
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
async fn delete_domain_in_workspace(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: DeleteDomainPath {
			workspace_id,
			domain_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<DeleteDomainRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Deleting domain in workspace", request_id);

	let user_id = token_data.user_id();

	let domain = db::get_workspace_domain_by_id(&mut connection, &domain_id)
		.await?
		.ok_or_else(|| ErrorType::NotFound)?;

	// TODO make sure all associated resources to this domain are removed first

	service::delete_domain_in_workspace(
		&mut connection,
		&workspace_id,
		&domain_id,
		&config,
		&request_id,
	)
	.await?;

	// Commiting transaction so that even if the mailing function fails the
	// resource should be deleted
	connection.commit().await?;

	service::resource_delete_action_email(
		&mut connection,
		&domain.name,
		&workspace_id,
		&ResourceType::Domain,
		&user_id,
	)
	.await?;

	log::trace!("request_id: {} - Deleted domain in workspace", request_id);
	// TODO: add the info to patr metrics
	Ok(())
}

async fn get_domain_dns_record(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: GetDomainDnsRecordsPath {
			workspace_id,
			domain_id,
		},
		query: Paginated {
			start,
			count,
			query: (),
		},
		body: (),
	}: DecodedRequest<GetDomainDnsRecordsRequest>,
) -> Result<GetDomainDnsRecordsResponse, Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Getting domain dns record", request_id);

	// get dns records from database
	let records = db::get_dns_records_by_domain_id(&mut connection, &domain_id)
		.await?
		.into_iter()
		.filter_map(|record| {
			let proxied = if let Some(proxied) = record.proxied {
				proxied
			} else {
				false
			};
			let record_value = match record.r#type {
				DnsRecordType::A => DnsRecordValue::A {
					target: record.value.parse().ok()?,
					proxied,
				},
				DnsRecordType::AAAA => DnsRecordValue::AAAA {
					target: record.value.parse().ok()?,
					proxied,
				},
				DnsRecordType::CNAME => DnsRecordValue::CNAME {
					target: record.value,
					proxied,
				},
				DnsRecordType::MX => DnsRecordValue::MX {
					target: record.value,
					priority: record.priority.map(|p| p as u16)?,
				},
				DnsRecordType::TXT => DnsRecordValue::TXT {
					target: record.value,
				},
			};
			Some(PatrDomainDnsRecord {
				id: record.id,
				domain_id: record.domain_id,
				name: record.name,
				r#type: record_value,
				ttl: record.ttl as u32,
			})
		})
		.collect();

	log::trace!("request_id: {} - Got domain dns record", request_id);
	Ok(GetDomainDnsRecordsResponse { records })
}

async fn add_dns_record(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: AddDnsRecordPath {
			workspace_id,
			domain_id,
		},
		query: (),
		body: AddDnsRecordRequest { name, r#type, ttl },
	}: DecodedRequest<AddDnsRecordRequest>,
) -> Result<AddDnsRecordResponse, Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Adding dns record", request_id);

	// add a record to cloudflare
	let record_id = service::create_patr_domain_dns_record(
		&mut connection,
		&workspace_id,
		&domain_id,
		&name.to_lowercase(),
		ttl,
		&r#type,
		&config,
		&request_id,
	)
	.await?;

	log::trace!("request_id: {} - Added dns record", request_id);
	Ok(AddDnsRecordResponse { id: record_id })
}

async fn update_dns_record(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path:
			UpdateDomainDnsRecordPath {
				workspace_id,
				domain_id,
				record_id,
			},
		query: (),
		body:
			UpdateDomainDnsRecordRequest {
				ttl,
				target,
				priority,
				proxied,
			},
	}: DecodedRequest<UpdateDomainDnsRecordRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Updating dns record", request_id);

	service::update_patr_domain_dns_record(
		&mut connection,
		&domain_id,
		&record_id,
		target.as_deref(),
		ttl,
		proxied,
		priority,
		&config,
		&request_id,
	)
	.await?;

	log::trace!("request_id: {} - Updated dns record", request_id);
	Ok(())
}

async fn delete_dns_record(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path:
			DeleteDnsRecordPath {
				workspace_id,
				domain_id,
				record_id,
			},
		query: (),
		body: (),
	}: DecodedRequest<DeleteDnsRecordRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {} - Deleting dns record", request_id);

	db::get_dns_record_by_id(&mut connection, &record_id)
		.await?
		.ok_or_else(|| ErrorType::NotFound)?;

	service::delete_patr_domain_dns_record(
		&mut connection,
		&domain_id,
		&record_id,
		&config,
		&request_id,
	)
	.await?;

	log::trace!("request_id: {} - Deleted dns record", request_id);
	Ok(())
}

async fn is_domain_personal(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: IsDomainPersonalPath { workspace_id },
		query: IsDomainPersonalRequest { domain },
		body: (),
	}: DecodedRequest<IsDomainPersonalRequest>,
) -> Result<IsDomainPersonalResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!(
		"request_id: {} - Checking if a domain is personal",
		request_id
	);

	let user_id = token_data.user_id();

	let (personal, is_used_by_others) = if let Some(domain) =
		db::get_domain_by_name(&mut connection, &domain).await?
	{
		if domain.r#type.is_personal() {
			(
				true,
				db::get_users_with_domain_in_personal_email(
					&mut connection,
					&domain.id,
				)
				.await?
				.into_iter()
				.any(|domain_user| domain_user != user_id),
			)
		} else {
			(false, false)
		}
	} else {
		(false, false)
	};

	log::trace!(
		"request_id: {} - Personal: {}, used by others: {}",
		request_id,
		personal,
		is_used_by_others
	);
	Ok(IsDomainPersonalResponse {
		personal,
		is_used_by_others,
	})
}
