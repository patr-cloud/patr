use std::net::{Ipv4Addr, Ipv6Addr};

use api_models::utils::Uuid;
use cloudflare::{
	endpoints::{
		dns::{
			CreateDnsRecord,
			CreateDnsRecordParams,
			DnsContent,
			UpdateDnsRecord,
			UpdateDnsRecordParams,
		},
		zone::{self, AccountId, CreateZone, CreateZoneParams, Status, Type},
	},
	framework::{
		async_api::{ApiClient, Client as CloudflareClient},
		auth::Credentials,
		Environment,
		HttpApiClientConfig,
	},
};
use eve_rs::AsError;
use tokio::{net::UdpSocket, task};
use trust_dns_client::{
	client::{AsyncClient, ClientHandle},
	rr::{rdata::TXT, DNSClass, Name, RData, RecordType},
	udp::UdpClientStream,
};

use crate::{
	db,
	error,
	models::{
		db_mapping::{DnsRecordType, DomainNameserverType},
		rbac,
	},
	utils::{
		constants::{request_keys, ResourceOwnerType},
		get_current_time_millis,
		settings::Settings,
		validator,
		Error,
	},
	Database,
};

/// # Description
/// This function checks if the personal domain exists, if it does not contain
/// domain this function will add the domain in the database and if the domain
/// is already present in workspace's table it will return an error
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `domain_name` - A string which contains domain name of user's personal
///   email id
///
/// # Returns
/// This function returns Result<Uuid, Error> which contains domain_id as uuid
/// or an error
///
///[`Transaction`]: Transaction
pub async fn ensure_personal_domain_exists(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_name: &str,
) -> Result<Uuid, Error> {
	if !validator::is_domain_name_valid(domain_name).await {
		Error::as_result()
			.status(400)
			.body(error!(INVALID_DOMAIN_NAME).to_string())?;
	}

	let domain = db::get_domain_by_name(connection, domain_name).await?;
	if let Some(domain) = domain {
		if let ResourceOwnerType::Business = domain.r#type {
			Error::as_result()
				.status(500)
				.body(error!(DOMAIN_BELONGS_TO_WORKSPACE).to_string())
		} else {
			Ok(domain.id)
		}
	} else {
		// check if personal domain given by the user is registerd as a
		// workspace domain
		if !is_domain_used_for_sign_up(connection, domain_name).await? {
			Error::as_result()
				.status(400)
				.body(error!(DOMAIN_BELONGS_TO_WORKSPACE).to_string())?;
		}

		let domain_id = db::generate_new_domain_id(connection).await?;
		db::create_generic_domain(
			connection,
			&domain_id,
			domain_name,
			&ResourceOwnerType::Personal,
		)
		.await?;

		db::add_to_personal_domain(connection, &domain_id).await?;

		Ok(domain_id)
	}
}

/// # Description
/// This function adds the workspace domain into the database
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `domain_name` - a string which contains domain name of user's personal
///   email id
/// * `workspace_id` - an unsigned 8 bit integer array which contains id of the
///   workspace
///
/// # Returns
/// This function returns Result<Uuid, Error> containing uuid of domain uuid or
/// an error
///
///[`Transaction`]: Transaction
pub async fn add_domain_to_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
	domain_name: &str,
	workspace_id: &Uuid,
	control_status: &DomainNameserverType,
) -> Result<Uuid, Error> {
	if !validator::is_domain_name_valid(domain_name).await {
		Error::as_result()
			.status(400)
			.body(error!(INVALID_DOMAIN_NAME).to_string())?;
	}

	let domain = db::get_domain_by_name(connection, domain_name).await?;
	if let Some(domain) = domain {
		if let ResourceOwnerType::Personal = domain.r#type {
			Error::as_result()
				.status(500)
				.body(error!(DOMAIN_IS_PERSONAL).to_string())?;
		} else {
			// check if personal domain given by the user is registerd as a
			// workspace domain
			if !is_domain_used_for_sign_up(connection, domain_name).await? {
				Error::as_result()
					.status(400)
					.body(error!(DOMAIN_EXISTS).to_string())?;
			}
		}
	}

	let domain_id = db::generate_new_domain_id(connection).await?;
	db::create_resource(
		connection,
		&domain_id,
		&format!("Domain: {}", domain_name),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::DOMAIN)
			.unwrap(),
		workspace_id,
		get_current_time_millis(),
	)
	.await?;
	db::create_generic_domain(
		connection,
		&domain_id,
		domain_name,
		&ResourceOwnerType::Business,
	)
	.await?;
	db::add_to_workspace_domain(connection, &domain_id, control_status).await?;
	if let DomainNameserverType::Internal = control_status {
		// create zone

		let client = get_cloudflare_client(config).await?;
		log::trace!("Creating zone for domain: {}", domain_name);
		// create zone
		let zone_identifier = client
			.request(&CreateZone {
				params: CreateZoneParams {
					name: domain_name,
					jump_start: Some(false),
					account: AccountId {
						id: &config.cloudflare.account_id,
					},
					// Full because the DNS record
					zone_type: Some(Type::Full),
				},
			})
			.await?
			.result
			.id;
		log::trace!("Zone created for domain: {}", domain_name);
		// create a new function to store zone related data
		db::add_patr_controlled_domain(
			connection,
			&domain_id,
			&zone_identifier,
		)
		.await?;
	}

	Ok(domain_id)
}

/// # Description
/// This function checks if the domain is verified or not
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `domain_id` - an unsigned 8 bit integer array containing id of
/// workspace domain
///
/// # Returns
/// Returns a Result<bool, Error> containing a bool whether the domain is
/// verified or not or an error
///
/// [`Transaction`]: Transaction
// TODO make domain store the registrar and
// NS servers and auto configure accordingly too
pub async fn is_domain_verified(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
	config: &Settings,
) -> Result<bool, Error> {
	let domain = db::get_workspace_domain_by_id(connection, domain_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	if let DomainNameserverType::Internal = domain.nameserver_type {
		let client = get_cloudflare_client(config).await?;

		let zone_identifier =
			db::get_patr_controlled_domain_by_domain_id(connection, domain_id)
				.await?
				.status(500)
				.body(error!(SERVER_ERROR).to_string())?
				.zone_identifier;

		let dns_record = client
			.request(&zone::ZoneDetails {
				identifier: &hex::encode(zone_identifier),
			})
			.await?;

		if let Status::Active = dns_record.result.status {
			db::update_workspace_domain_status(connection, domain_id, true)
				.await?;
			return Ok(true);
		}

		Ok(false)
	} else {
		// TODO: make domain name server a config instead of string
		let (mut client, bg) =
			AsyncClient::connect(UdpClientStream::<UdpSocket>::new(
				request_keys::DNS_RESOLVER.parse()?,
			))
			.await?;

		let handle = task::spawn(bg);
		let mut response = client
			.query(
				Name::from_utf8(format!("PatrVerify.{}", domain.name))?,
				DNSClass::IN,
				RecordType::TXT,
			)
			.await?;

		let response = response.take_answers().into_iter().find(|record| {
			let expected_txt =
				RData::TXT(TXT::new(vec![domain.id.to_string()]));
			record.rdata() == &expected_txt
		});

		handle.abort();
		drop(handle);

		if response.is_some() {
			db::update_workspace_domain_status(connection, domain_id, true)
				.await?;

			return Ok(true);
		}

		Ok(false)
	}
}

/// # Description
/// This function is used to check if the workspace domain was used during
/// the sign up or not
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `domain_name` - a string containing name of the workspace domain
///
/// # Returns
/// Returns a Result<bool, Error> containing a bool whether the domain is
/// used for sign up or not or an error
///
/// [`Transaction`]: Transaction
async fn is_domain_used_for_sign_up(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_name: &str,
) -> Result<bool, Error> {
	let workspace_domain_status =
		db::get_user_to_sign_up_by_business_domain_name(
			connection,
			domain_name,
		)
		.await?;
	if let Some(workspace_domain_status) = workspace_domain_status {
		if workspace_domain_status.otp_expiry > get_current_time_millis() {
			return Ok(false);
		}
	}
	Ok(true)
}

// SERVICE FUNCTIONS FOR DNS RECORD

pub async fn add_patr_dns_record(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	domain_id: &Uuid,
	zone_identifier: &str,
	name: &str,
	record: &str,
	ttl: u32,
	proxied: bool,
	priority: Option<u64>,
	dns_record_type: &DnsRecordType,
	config: &Settings,
) -> Result<Uuid, Error> {
	// login to cloudflare to create new DNS record cloudflare
	let client = get_cloudflare_client(config).await?;

	let dns_id = db::generate_new_resource_id(connection).await?;
	log::trace!("creating resource");

	db::create_resource(
		connection,
		&dns_id,
		&format!("DNS Record `{}.{}`: {}", name, domain_id, dns_record_type),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::DNS_RECORD)
			.unwrap(),
		workspace_id,
		get_current_time_millis(),
	)
	.await?;

	// add to db
	db::create_patr_domain_dns_record(
		connection,
		&dns_id,
		domain_id,
		name,
		dns_record_type,
		record,
		None,
		ttl as i32,
		proxied,
	)
	.await?;

	match dns_record_type {
		DnsRecordType::A => {
			// Cloudflare api takes content as Ipv4 object.
			let a_record_ipv4 = record.parse::<Ipv4Addr>()?;

			// send request to Cloudflare
			client
				.request(&CreateDnsRecord {
					zone_identifier,
					params: CreateDnsRecordParams {
						ttl: Some(ttl),
						priority: None,
						proxied: Some(proxied),
						name,
						content: DnsContent::A {
							content: a_record_ipv4,
						},
					},
				})
				.await?;
		}
		DnsRecordType::Aaaa => {
			let ipv6 = record.parse::<Ipv6Addr>()?;

			// send request to Cloudflare
			client
				.request(&CreateDnsRecord {
					zone_identifier,
					params: CreateDnsRecordParams {
						ttl: Some(ttl),
						priority: None,
						proxied: Some(proxied),
						name,
						content: DnsContent::AAAA { content: ipv6 },
					},
				})
				.await?;
		}
		DnsRecordType::Mx => {
			// send request to Cloudflare
			if let Some(priority) = priority {
				client
					.request(&CreateDnsRecord {
						zone_identifier,
						params: CreateDnsRecordParams {
							ttl: Some(ttl),
							priority: None,
							proxied: Some(proxied),
							name,
							content: DnsContent::MX {
								priority: priority as u16,
								content: record.to_string(),
							},
						},
					})
					.await?;
			} else {
				return Error::as_result()
					.status(400)
					.body(error!(WRONG_PARAMETERS).to_string())?;
			}
		}
		DnsRecordType::Cname => {
			// send request to Cloudflare
			client
				.request(&CreateDnsRecord {
					zone_identifier,
					params: CreateDnsRecordParams {
						ttl: Some(ttl),
						priority: None,
						proxied: Some(proxied),
						name,
						content: DnsContent::CNAME {
							content: record.to_string(),
						},
					},
				})
				.await?;
		}
		DnsRecordType::Txt => {
			// send request to Cloudflare
			client
				.request(&CreateDnsRecord {
					zone_identifier,
					params: CreateDnsRecordParams {
						ttl: Some(ttl),
						priority: None,
						proxied: Some(proxied),
						name,
						content: DnsContent::TXT {
							content: record.to_string(),
						},
					},
				})
				.await?;
		}
	}

	Ok(dns_id)
}

pub async fn update_patr_dns_record(
	connection: &mut <Database as sqlx::Database>::Connection,
	dns_id: &Uuid,
	zone_identifier: &str,
	record: Option<&str>,
	ttl: Option<u32>,
	proxied: Option<bool>,
	priority: Option<u16>,
	dns_record_type: &DnsRecordType,
	config: &Settings,
) -> Result<(), Error> {
	// login to cloudflare to create new DNS record cloudflare
	let client = get_cloudflare_client(config).await?;

	let dns_record = db::get_dns_record_by_id(connection, dns_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	db::update_patr_domain_dns_record(
		connection,
		dns_id,
		record,
		priority.map(|p| p as i32),
		ttl.map(|ttl| ttl as i32),
		proxied,
	)
	.await?;

	let record = if let Some(record) = record {
		record
	} else {
		&dns_record.value
	};

	match dns_record_type {
		DnsRecordType::A => {
			let record = record.parse::<Ipv4Addr>()?;

			// send request to Cloudflare
			client
				.request(&UpdateDnsRecord {
					zone_identifier,
					params: UpdateDnsRecordParams {
						ttl,
						proxied,
						name: &dns_record.name,
						content: DnsContent::A { content: record },
					},
					identifier: zone_identifier,
				})
				.await?;
		}
		DnsRecordType::Aaaa => {
			let record = record.parse::<Ipv6Addr>()?;

			// send request to Cloudflare
			client
				.request(&UpdateDnsRecord {
					zone_identifier,
					params: UpdateDnsRecordParams {
						ttl,
						proxied,
						name: &dns_record.name,
						content: DnsContent::AAAA { content: record },
					},
					identifier: zone_identifier,
				})
				.await?;
		}
		DnsRecordType::Mx => {
			let priority = if let Some(priority) = priority {
				priority
			} else {
				dns_record.priority.map(|p| p as u16).status(500)?
			};

			// send request to Cloudflare
			client
				.request(&UpdateDnsRecord {
					zone_identifier,
					params: UpdateDnsRecordParams {
						ttl,
						proxied,
						name: &dns_record.name,
						content: DnsContent::MX {
							content: record.to_string(),
							priority,
						},
					},
					identifier: zone_identifier,
				})
				.await?;
		}
		DnsRecordType::Cname => {
			// send request to Cloudflare
			client
				.request(&UpdateDnsRecord {
					zone_identifier,
					params: UpdateDnsRecordParams {
						ttl,
						proxied,
						name: &dns_record.name,
						content: DnsContent::CNAME {
							content: record.to_string(),
						},
					},
					identifier: zone_identifier,
				})
				.await?;
		}
		DnsRecordType::Txt => {
			// send request to Cloudflare
			client
				.request(&UpdateDnsRecord {
					zone_identifier,
					params: UpdateDnsRecordParams {
						ttl,
						proxied,
						name: &dns_record.name,
						content: DnsContent::TXT {
							content: record.to_string(),
						},
					},
					identifier: zone_identifier,
				})
				.await?;
		}
	}
	Ok(())
}

pub async fn get_cloudflare_client(
	config: &Settings,
) -> Result<CloudflareClient, Error> {
	// login to cloudflare and create zone in cloudflare
	let credentials = Credentials::UserAuthToken {
		token: config.cloudflare.api_token.clone(),
	};

	let client = if let Ok(client) = CloudflareClient::new(
		credentials,
		HttpApiClientConfig::default(),
		Environment::Production,
	) {
		client
	} else {
		return Err(Error::empty());
	};
	Ok(client)
}
