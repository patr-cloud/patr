use std::net::{Ipv4Addr, Ipv6Addr};

use api_models::{
	models::workspace::domain::{DnsRecordValue, DomainNameserverType},
	utils::{DateTime, ResourceType, Uuid},
};
use chrono::Utc;
use cloudflare::{
	endpoints::{
		dns::{
			CreateDnsRecord,
			CreateDnsRecordParams,
			DeleteDnsRecord,
			DnsContent,
			UpdateDnsRecord,
			UpdateDnsRecordParams,
		},
		zone::{
			AccountId,
			CreateZone,
			CreateZoneParams,
			DeleteZone,
			Status,
			Type,
			ZoneDetails,
		},
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

use super::infrastructure;
use crate::{
	db::{self, DnsRecordType, DomainPlan},
	error,
	models::rbac::{self, resource_types},
	utils::{
		constants,
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
//TODO: add log statements
pub async fn ensure_personal_domain_exists(
	connection: &mut <Database as sqlx::Database>::Connection,
	full_domain_name: &str,
) -> Result<Uuid, Error> {
	let (domain_name, tld) = super::split_domain_and_tld(full_domain_name)
		.await
		.status(400)
		.body(error!(INVALID_DOMAIN_NAME).to_string())?;
	let (domain_name, tld) = (domain_name.as_str(), tld.as_str());

	let domain = db::get_domain_by_name(connection, full_domain_name).await?;
	if let Some(domain) = domain {
		if let ResourceType::Business = domain.r#type {
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
			tld,
			&ResourceType::Personal,
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
	full_domain_name: &str,
	nameserver_type: &DomainNameserverType,
	workspace_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<Uuid, Error> {
	log::trace!(
		"request_id: {} - Splitting the domain name and TLD",
		request_id
	);
	let (domain_name, tld) = super::split_domain_and_tld(full_domain_name)
		.await
		.status(400)
		.body(error!(INVALID_DOMAIN_NAME).to_string())?;
	let (domain_name, tld) = (domain_name.as_str(), tld.as_str());

	log::trace!(
		"request_id: {} - Checking if the domain is already used",
		request_id
	);
	let domain = db::get_domain_by_name(connection, full_domain_name).await?;
	if let Some(domain) = domain {
		if let ResourceType::Personal = domain.r#type {
			Error::as_result()
				.status(500)
				.body(error!(DOMAIN_IS_PERSONAL).to_string())?;
		} else {
			// check if personal domain given by the user is registerd as a
			// workspace domain
			if !is_domain_used_for_sign_up(connection, full_domain_name).await?
			{
				Error::as_result()
					.status(400)
					.body(error!(DOMAIN_EXISTS).to_string())?;
			}
		}
	}

	log::trace!("request_id: {} - Generating new domain id", request_id);
	let domain_id = db::generate_new_domain_id(connection).await?;

	log::trace!("request_id: {} - Checking resource limit", request_id);
	if super::resource_limit_crossed(connection, workspace_id, request_id)
		.await?
	{
		return Error::as_result()
			.status(400)
			.body(error!(RESOURCE_LIMIT_EXCEEDED).to_string())?;
	}

	log::trace!("request_id: {} - Checking static site limit", request_id);
	if domain_limit_crossed(connection, workspace_id, request_id).await? {
		return Error::as_result()
			.status(400)
			.body(error!(DOMAIN_LIMIT_EXCEEDED).to_string())?;
	}

	let creation_time = Utc::now();
	log::trace!("request_id: {} - Generating new resource", request_id);
	db::create_resource(
		connection,
		&domain_id,
		&format!("Domain: {}", full_domain_name),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::DOMAIN)
			.unwrap(),
		workspace_id,
		creation_time.timestamp_millis() as u64,
	)
	.await?;
	db::create_generic_domain(
		connection,
		&domain_id,
		domain_name,
		tld,
		&ResourceType::Business,
	)
	.await?;

	log::trace!("request_id: {} - Adding domain to workspace", request_id);
	db::add_to_workspace_domain(connection, &domain_id, nameserver_type)
		.await?;

	let domain_plan =
		match db::get_domains_for_workspace(connection, workspace_id)
			.await?
			.len()
		{
			(0..=1) => DomainPlan::Free,
			(2..) => DomainPlan::Unlimited,
			_ => unreachable!(),
		};
	db::update_domain_usage_history(
		connection,
		workspace_id,
		&domain_plan,
		&DateTime::from(creation_time),
	)
	.await?;

	if nameserver_type.is_internal() {
		log::trace!(
			"request_id: {} - Adding domain to internal nameserver",
			request_id
		);
		if ["cf", "ga", "gq", "ml", "tk"].contains(&tld) {
			return Err(Error::empty()
				.status(400)
				.body(error!(INVALID_DOMAIN_NAME).to_string()));
		}
		// create zone
		let client = get_cloudflare_client(config).await?;
		log::trace!(
			"request_id: {} - Creating zone for domain: {}",
			request_id,
			full_domain_name
		);
		// create zone
		let zone_identifier = client
			.request(&CreateZone {
				params: CreateZoneParams {
					name: full_domain_name,
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
		log::trace!("Zone created for domain: {}", full_domain_name);
		// create a new function to store zone related data
		db::add_patr_controlled_domain(
			connection,
			&domain_id,
			&zone_identifier,
		)
		.await?;
	} else {
		db::add_user_controlled_domain(connection, &domain_id).await?;
	}

	log::trace!("request_id: {} - Domain added successfully", request_id);
	Ok(domain_id)
}

/// # Description
/// This function transfers the domain ownership from user to patr
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `domain_name` - a string which contains domain name of user's personal
///   email id
///
/// # Returns
/// This function returns Result<(), Error> containing succes or
/// an error
///
///[`Transaction`]: Transaction

pub async fn transfer_domain_to_patr(
	connection: &mut <Database as sqlx::Database>::Connection,
	_workspace_id: &Uuid,
	domain: &str,
	config: &Settings,
	_request_id: &Uuid,
) -> Result<(), Error> {
	let client = get_cloudflare_client(config).await?;

	let domain = db::get_domain_by_name(connection, domain)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	// create zone
	let zone_identifier = client
		.request(&CreateZone {
			params: CreateZoneParams {
				name: domain.name.as_str(),
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

	let user_controlled_domain =
		db::get_user_controlled_domain_by_name(connection, &domain.id).await?;

	// Insert into patr_controlled_domains
	db::add_patr_controlled_domain(
		connection,
		&user_controlled_domain.domain_id,
		&zone_identifier,
	)
	.await?;

	// Delete from user_controlled_domains
	// TODO - check if deleting this will mess up something else
	db::delete_user_contolled_domain(
		connection,
		&user_controlled_domain.domain_id,
	)
	.await?;

	// Update workspace_domain with with nameserver_typea as internal
	db::update_workspace_domain_nameserver_type(
		connection,
		&user_controlled_domain.domain_id,
	)
	.await?;

	// TODO - manage certs and secrets

	Ok(())
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
	workspace_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<bool, Error> {
	log::trace!(
		"request_id: {} - Checking if domain is verified",
		request_id
	);
	let domain = db::get_workspace_domain_by_id(connection, domain_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	if domain.is_verified {
		return Ok(true);
	}

	if domain.is_ns_internal() {
		log::trace!("request_id: {} - Domain is internal", request_id);
		let client = get_cloudflare_client(config).await?;

		let zone_identifier =
			db::get_patr_controlled_domain_by_id(connection, domain_id)
				.await?
				.status(500)
				.body(error!(SERVER_ERROR).to_string())?
				.zone_identifier;

		log::trace!("request_id: {} - Checking if zone is active", request_id);
		let zone = client
			.request(&ZoneDetails {
				identifier: &zone_identifier,
			})
			.await?;

		if let Status::Active = zone.result.status {
			log::trace!("request_id: {} - Zone is active", request_id);
			log::trace!(
				"request_id: {} - Updating domain verification status",
				request_id
			);
			db::update_workspace_domain_status(connection, domain_id, true)
				.await?;

			log::trace!("request_id: {} - Creating wild card certiifcate for internal domain", request_id);
			infrastructure::create_certificates(
				workspace_id,
				&format!("certificate-{}", domain_id),
				&format!("tls-{}", domain_id),
				vec![format!("*.{}", domain.name), domain.name.clone()],
				true,
				config,
				request_id,
			)
			.await?;
			log::trace!("request_id: {} - Domain verified", request_id);
			return Ok(true);
		}

		Ok(false)
	} else {
		log::trace!("request_id: {} - Domain is not internal", request_id);
		log::trace!("request_id: {} - Verifying external domain", request_id);
		verify_external_domain(
			connection,
			workspace_id,
			&domain.name,
			&domain.id,
			config,
			request_id,
		)
		.await
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
pub async fn is_domain_used_for_sign_up(
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

pub async fn create_patr_domain_dns_record(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	domain_id: &Uuid,
	name: &str,
	ttl: u32,
	dns_record: &DnsRecordValue,
	config: &Settings,
	request_id: &Uuid,
) -> Result<Uuid, Error> {
	// check if domain is patr controlled
	log::trace!("request_id: {} - Checking if name is valid", request_id);

	if !validator::is_dns_record_name_valid(name) {
		Error::as_result()
			.status(200)
			.body(error!(INVALID_DNS_RECORD_NAME).to_string())?;
	};

	log::trace!(
		"request_id: {} - Checking if domain is patr controlled",
		request_id
	);
	let domain = db::get_patr_controlled_domain_by_id(connection, domain_id)
		.await?
		.status(400)
		.body(error!(DOMAIN_NOT_PATR_CONTROLLED).to_string())?;

	// login to cloudflare to create new DNS record cloudflare
	let client = get_cloudflare_client(config).await?;

	log::trace!("request_id: {} - Creating domain id", request_id);
	let record_id = db::generate_new_resource_id(connection).await?;

	log::trace!("request_id: {} - Getting dns record type", request_id);
	let (record, priority, dns_record_type, proxied) = match dns_record {
		DnsRecordValue::A { target, proxied } => {
			(target.to_string(), None, DnsRecordType::A, Some(*proxied))
		}
		DnsRecordValue::MX { target, priority } => {
			(target.clone(), Some(priority), DnsRecordType::MX, None)
		}
		DnsRecordValue::TXT { target } => {
			(target.clone(), None, DnsRecordType::TXT, None)
		}
		DnsRecordValue::AAAA { target, proxied } => (
			target.to_string(),
			None,
			DnsRecordType::AAAA,
			Some(*proxied),
		),
		DnsRecordValue::CNAME { target, proxied } => {
			(target.clone(), None, DnsRecordType::CNAME, Some(*proxied))
		}
	};

	log::trace!("request_id: {} - Generating new resource", request_id);
	db::create_resource(
		connection,
		&record_id,
		&format!("DNS Record `{}.{}`: {}", name, domain_id, dns_record_type),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(resource_types::DNS_RECORD)
			.unwrap(),
		workspace_id,
		get_current_time_millis(),
	)
	.await?;

	log::trace!("request_id: {} - Parsing Dns record type", request_id);
	let content = match dns_record {
		DnsRecordValue::A { target, .. } => DnsContent::A { content: *target },
		DnsRecordValue::MX { target, priority } => DnsContent::MX {
			priority: *priority,
			content: target.clone(),
		},
		DnsRecordValue::TXT { target } => DnsContent::TXT {
			content: target.clone(),
		},
		DnsRecordValue::AAAA { target, .. } => {
			DnsContent::AAAA { content: *target }
		}
		DnsRecordValue::CNAME { target, .. } => DnsContent::CNAME {
			content: target.clone(),
		},
	};

	// add to db
	log::trace!("request_id: {} - Adding to db", request_id);
	db::create_patr_domain_dns_record(
		connection,
		&record_id,
		Uuid::nil().as_str(),
		domain_id,
		name,
		&dns_record_type,
		&record,
		priority.map(|p| *p as i32),
		ttl as i64,
		proxied,
	)
	.await?;

	// send request to Cloudflare
	log::trace!("request_id: {} - Sending request to Cloudflare for creating DNS record", request_id);
	let dns_identifier = client
		.request(&CreateDnsRecord {
			zone_identifier: &domain.zone_identifier,
			params: CreateDnsRecordParams {
				ttl: Some(ttl),
				priority: None,
				proxied,
				name,
				content,
			},
		})
		.await?
		.result
		.id;
	log::trace!(
		"request_id: {} - Created DNS record id: {}",
		request_id,
		dns_identifier
	);

	log::trace!("request_id: {} - Updating patr domain dns record with record identifier", request_id);

	db::update_dns_record_identifier(connection, &record_id, &dns_identifier)
		.await?;

	log::trace!(
		"request_id: {} - Created DNS record id: {}",
		request_id,
		dns_identifier
	);
	Ok(record_id)
}

pub async fn update_patr_domain_dns_record(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
	record_id: &Uuid,
	target: Option<&str>,
	ttl: Option<u32>,
	proxied: Option<bool>,
	priority: Option<u16>,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!("request_id: {} - Checking if domain exists", request_id);
	let domain = db::get_patr_controlled_domain_by_id(connection, domain_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	log::trace!("request_id: {} - Checking if DNS record exists", request_id);
	let dns_record = db::get_dns_record_by_id(connection, record_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	if dns_record.r#type != DnsRecordType::MX && priority.is_some() {
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	}

	// login to cloudflare to create new DNS record cloudflare
	let client = get_cloudflare_client(config).await?;

	log::trace!("request_id: {} - Updating DNS record in the db", request_id);
	db::update_patr_domain_dns_record(
		connection,
		record_id,
		target,
		priority.map(|p| p as i32),
		ttl.map(|ttl| ttl as i64),
		proxied,
	)
	.await?;

	let record = if let Some(record) = target {
		record
	} else {
		&dns_record.value
	};

	let content = match dns_record.r#type {
		DnsRecordType::A => DnsContent::A {
			content: record.parse::<Ipv4Addr>()?,
		},
		DnsRecordType::AAAA => DnsContent::AAAA {
			content: record.parse::<Ipv6Addr>()?,
		},
		DnsRecordType::MX => DnsContent::MX {
			priority: priority
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())? as u16,
			content: record.to_string(),
		},
		DnsRecordType::CNAME => DnsContent::CNAME {
			content: record.to_string(),
		},
		DnsRecordType::TXT => DnsContent::TXT {
			content: record.to_string(),
		},
	};

	// send request to Cloudflare
	log::trace!("request_id: {} - Sending request to Cloudflare for updating DNS record", request_id);
	client
		.request(&UpdateDnsRecord {
			zone_identifier: &domain.zone_identifier,
			params: UpdateDnsRecordParams {
				ttl,
				proxied,
				name: &dns_record.name,
				content,
			},
			identifier: &dns_record.record_identifier,
		})
		.await?;

	log::trace!(
		"request_id: {} - Updated DNS record id: {}",
		request_id,
		dns_record.record_identifier
	);
	Ok(())
}

pub async fn delete_patr_domain_dns_record(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
	record_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!("request_id: {} - Checking if domain exists", request_id);
	// check if domain is patr controlled
	let domain = db::get_patr_controlled_domain_by_id(connection, domain_id)
		.await?
		.status(400)
		.body(error!(DOMAIN_NOT_PATR_CONTROLLED).to_string())?;

	log::trace!("request_id: {} - Checking if DNS record exists", request_id);
	let dns_record = db::get_dns_record_by_id(connection, record_id)
		.await?
		.status(400)
		.body(error!(DNS_RECORD_NOT_FOUND).to_string())?;

	log::trace!("request_id: {} - Deleting DNS record in the db", request_id);
	db::delete_patr_controlled_dns_record(connection, record_id).await?;

	let client = get_cloudflare_client(config).await?;

	log::trace!("request_id: {} - Sending request to Cloudflare for deleting DNS record", request_id);
	client
		.request(&DeleteDnsRecord {
			identifier: &dns_record.record_identifier,
			zone_identifier: &domain.zone_identifier,
		})
		.await?;

	log::trace!("request_id: {} - Deleted DNS record", request_id);
	Ok(())
}

pub async fn verify_external_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	domain_name: &str,
	domain_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<bool, Error> {
	log::trace!("request_id: {} - Getting the client", request_id);
	let (mut client, bg) = AsyncClient::connect(
		UdpClientStream::<UdpSocket>::new(constants::DNS_RESOLVER.parse()?),
	)
	.await?;

	let handle = task::spawn(bg);

	log::trace!("request_id: {} - querying the DNS server to check if the TXT record exists or not", request_id);
	let mut response = client
		.query(
			Name::from_utf8(format!("patrVerify.{}", domain_name))?,
			DNSClass::IN,
			RecordType::TXT,
		)
		.await?;

	let expected_txt = RData::TXT(TXT::new(vec![domain_id.to_string()]));
	let response = response
		.take_answers()
		.into_iter()
		.find(|record| record.data() == Some(&expected_txt));

	handle.abort();
	drop(handle);

	if response.is_some() {
		log::trace!("request_id: {} - The TXT record exists", request_id);
		log::trace!(
			"request_id: {} - Creating the certificate for managed url",
			request_id
		);
		create_certificates_of_managed_urls_for_domain(
			connection,
			workspace_id,
			domain_id,
			domain_name,
			config,
			request_id,
		)
		.await?;

		log::trace!("request_id: {} - Verified the domain and updating workspace domain status", request_id);
		db::update_workspace_domain_status(connection, domain_id, true).await?;

		return Ok(true);
	}

	Ok(false)
}

pub async fn delete_domain_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	domain_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!("request_id: {} - Deleting the domain in the db", request_id);
	let domain = db::get_workspace_domain_by_id(connection, domain_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	log::trace!("request_id: {} - Checking if there are any active managed urls for the domain or not", request_id);
	let managed_urls_count =
		db::get_active_managed_url_count_for_domain(connection, domain_id)
			.await?;
	if managed_urls_count > 0 {
		return Err(Error::empty()
			.status(400)
			.body(error!(RESOURCE_IN_USE).to_string()));
	}

	log::trace!(
		"request_id: {} - Updating the domain name in the db",
		request_id
	);
	db::update_workspace_domain_status(connection, &domain.id, false).await?;
	db::update_generic_domain_name(
		connection,
		&domain.id,
		&format!("patr-deleted: {}@{}", domain.id, domain.name),
	)
	.await?;
	db::update_resource_name(
		connection,
		&domain.id,
		&format!("Domain: patr-deleted: {}@{}", domain.id, domain.name),
	)
	.await?;

	let domain_plan =
		match db::get_domains_for_workspace(connection, workspace_id)
			.await?
			.len()
		{
			(0..=1) => DomainPlan::Free,
			(2..) => DomainPlan::Unlimited,
			_ => unreachable!(),
		};
	db::update_domain_usage_history(
		connection,
		workspace_id,
		&domain_plan,
		&DateTime::from(Utc::now()),
	)
	.await?;

	if domain.is_ns_internal() {
		log::trace!(
			"request_id: {} - Getting the information for the internal domain",
			request_id
		);
		let domain =
			db::get_patr_controlled_domain_by_id(connection, &domain.id)
				.await?
				.status(500)
				.body(error!(SERVER_ERROR).to_string())?;

		log::trace!("request_id: {} - Checking if there are any DNS records present for the domain", request_id);
		let dns_record_count =
			db::get_dns_record_count_for_domain(connection, &domain.domain_id)
				.await?;
		if dns_record_count > 0 {
			return Err(Error::empty()
				.status(400)
				.body(error!(RESOURCE_IN_USE).to_string()));
		}

		let secret_name = format!("tls-{}", domain.domain_id);
		let certificate_name = format!("certificate-{}", domain.domain_id);
		log::trace!(
			"request_id: {} - Deleting the certificate from db",
			request_id
		);
		infrastructure::delete_certificates_for_domain(
			workspace_id,
			&certificate_name,
			&secret_name,
			config,
			request_id,
		)
		.await?;

		// delete cloudflare zone
		log::trace!(
			"request_id: {} - Deleting the cloudflare zone",
			request_id
		);
		let client = get_cloudflare_client(config).await?;
		client
			.request(&DeleteZone {
				identifier: &domain.zone_identifier,
			})
			.await?;
	}
	log::trace!("request_id: {} - Domain deleted successfully", request_id);
	Ok(())
}

pub async fn create_certificates_of_managed_urls_for_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	domain_id: &Uuid,
	domain_name: &str,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - Getting the managed urls for the domain",
		request_id
	);
	let managed_urls =
		db::get_all_managed_urls_for_domain(connection, domain_id).await?;

	log::trace!(
		"request_id: {} - Creating the certificates for the managed urls",
		request_id
	);
	for managed_url in managed_urls {
		infrastructure::create_certificates(
			workspace_id,
			&format!("certificate-{}", managed_url.id),
			&format!("tls-{}", managed_url.id),
			vec![format!("{}.{}", managed_url.sub_domain, domain_name)],
			false,
			config,
			request_id,
		)
		.await?;
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

async fn domain_limit_crossed(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	request_id: &Uuid,
) -> Result<bool, Error> {
	log::trace!(
		"request_id: {} - Checking if free limits are crossed",
		request_id
	);

	let workspace = db::get_workspace_info(connection, workspace_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let current_domains =
		db::get_domains_for_workspace(connection, workspace_id)
			.await?
			.len();

	log::trace!(
		"request_id: {} - Checking if domains limits are crossed",
		request_id
	);
	if current_domains + 1 > workspace.domain_limit as usize {
		return Ok(true);
	}

	Ok(false)
}
