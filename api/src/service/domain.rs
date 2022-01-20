use std::net::{Ipv4Addr, Ipv6Addr};

use api_models::{
	models::workspace::domain::{DnsRecordValue, DomainNameserverType},
	utils::{ResourceType, Uuid},
};
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
	db,
	error,
	models::{
		db_mapping::DnsRecordType,
		rbac::{self, resource_types},
	},
	utils::{
		constants::{self},
		get_current_time_millis,
		settings::Settings,
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
	let (domain_name, tld) = super::split_domain_and_tld(domain_name)
		.await
		.status(400)
		.body(error!(INVALID_DOMAIN_NAME).to_string())?;
	let (domain_name, tld) = (domain_name.as_str(), tld.as_str());

	let domain = db::get_domain_by_name(connection, domain_name).await?;
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
	domain_name: &str,
	nameserver_type: &DomainNameserverType,
	workspace_id: &Uuid,
	config: &Settings,
) -> Result<Uuid, Error> {
	let (domain_name, tld) = super::split_domain_and_tld(domain_name)
		.await
		.status(400)
		.body(error!(INVALID_DOMAIN_NAME).to_string())?;
	let (domain_name, tld) = (domain_name.as_str(), tld.as_str());

	let domain = db::get_domain_by_name(connection, domain_name).await?;
	if let Some(domain) = domain {
		if let ResourceType::Personal = domain.r#type {
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
		tld,
		&ResourceType::Business,
	)
	.await?;
	db::add_to_workspace_domain(connection, &domain_id, nameserver_type)
		.await?;

	if nameserver_type.is_internal() {
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
	} else {
		db::add_user_controlled_domain(connection, &domain_id).await?;
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
	workspace_id: &Uuid,
	config: &Settings,
) -> Result<bool, Error> {
	let domain = db::get_workspace_domain_by_id(connection, domain_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	if domain.is_verified {
		return Ok(true);
	}

	if domain.is_ns_internal() {
		let client = get_cloudflare_client(config).await?;

		let zone_identifier =
			db::get_patr_controlled_domain_by_id(connection, domain_id)
				.await?
				.status(500)
				.body(error!(SERVER_ERROR).to_string())?
				.zone_identifier;

		let zone = client
			.request(&ZoneDetails {
				identifier: &zone_identifier,
			})
			.await?;

		if let Status::Active = zone.result.status {
			db::update_workspace_domain_status(connection, domain_id, true)
				.await?;

			infrastructure::create_certificates(
				workspace_id,
				&format!("certificate-{}", domain_id),
				&format!("tls-{}", domain_id),
				vec![format!("*.{}", domain.name), domain.name],
				config,
			)
			.await?;
			return Ok(true);
		}

		Ok(false)
	} else {
		Ok(verify_external_domain(
			connection,
			workspace_id,
			&domain.name,
			&domain.id,
			config,
		)
		.await?)
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
	proxied: bool,
	dns_record: &DnsRecordValue,
	config: &Settings,
) -> Result<Uuid, Error> {
	// check if domain is patr controlled
	let domain = db::get_patr_controlled_domain_by_id(connection, domain_id)
		.await?
		.status(400)
		.body(error!(DOMAIN_NOT_PATR_CONTROLLED).to_string())?;

	// login to cloudflare to create new DNS record cloudflare
	let client = get_cloudflare_client(config).await?;
	log::trace!("creating resource");

	let record_id = db::generate_new_resource_id(connection).await?;

	let (record, priority, dns_record_type) = match dns_record {
		DnsRecordValue::A { target } => (target, None, DnsRecordType::A),
		DnsRecordValue::MX { target, priority } => {
			(target, Some(priority), DnsRecordType::MX)
		}
		DnsRecordValue::TXT { target } => (target, None, DnsRecordType::TXT),
		DnsRecordValue::AAAA { target } => (target, None, DnsRecordType::AAAA),
		DnsRecordValue::CNAME { target } => {
			(target, None, DnsRecordType::CNAME)
		}
	};

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

	let content = match dns_record {
		DnsRecordValue::A { target } => DnsContent::A {
			content: target.parse::<Ipv4Addr>()?,
		},
		DnsRecordValue::MX { target, priority } => DnsContent::MX {
			priority: *priority,
			content: target.clone(),
		},
		DnsRecordValue::TXT { target } => DnsContent::TXT {
			content: target.clone(),
		},
		DnsRecordValue::AAAA { target } => DnsContent::AAAA {
			content: target.parse::<Ipv6Addr>()?,
		},
		DnsRecordValue::CNAME { target } => DnsContent::CNAME {
			content: target.clone(),
		},
	};

	// send request to Cloudflare
	let dns_identifier = client
		.request(&CreateDnsRecord {
			zone_identifier: &domain.zone_identifier,
			params: CreateDnsRecordParams {
				ttl: Some(ttl),
				priority: None,
				proxied: Some(proxied),
				name,
				content,
			},
		})
		.await?
		.result
		.id;

	// add to db
	db::create_patr_domain_dns_record(
		connection,
		&record_id,
		&dns_identifier,
		domain_id,
		name,
		&dns_record_type,
		record,
		priority.map(|p| *p as i32),
		ttl as i64,
		proxied,
	)
	.await?;

	Ok(record_id)
}

pub async fn update_patr_domain_dns_record(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
	record_id: &Uuid,
	content: Option<&str>,
	ttl: Option<u32>,
	proxied: Option<bool>,
	priority: Option<u16>,
	config: &Settings,
) -> Result<(), Error> {
	let domain = db::get_patr_controlled_domain_by_id(connection, domain_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

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

	db::update_patr_domain_dns_record(
		connection,
		record_id,
		content,
		priority.map(|p| p as i32),
		ttl.map(|ttl| ttl as i64),
		proxied,
	)
	.await?;

	let record = if let Some(record) = content {
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
	Ok(())
}

pub async fn delete_patr_domain_dns_record(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
	record_id: &Uuid,
	config: &Settings,
) -> Result<(), Error> {
	// check if domain is patr controlled
	let domain = db::get_patr_controlled_domain_by_id(connection, domain_id)
		.await?
		.status(400)
		.body(error!(DOMAIN_NOT_PATR_CONTROLLED).to_string())?;

	let dns_record = db::get_dns_record_by_id(connection, record_id)
		.await?
		.status(400)
		.body(error!(DNS_RECORD_NOT_FOUND).to_string())?;

	db::delete_patr_controlled_dns_record(connection, record_id).await?;

	let client = get_cloudflare_client(config).await?;

	client
		.request(&DeleteDnsRecord {
			identifier: &dns_record.record_identifier,
			zone_identifier: &domain.zone_identifier,
		})
		.await?;

	Ok(())
}

pub async fn verify_external_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	domain_name: &str,
	domain_id: &Uuid,
	config: &Settings,
) -> Result<bool, Error> {
	let (mut client, bg) = AsyncClient::connect(
		UdpClientStream::<UdpSocket>::new(constants::DNS_RESOLVER.parse()?),
	)
	.await?;

	let handle = task::spawn(bg);
	let mut response = client
		.query(
			Name::from_utf8(format!("patrVerify.{}", domain_name))?,
			DNSClass::IN,
			RecordType::TXT,
		)
		.await?;

	let response = response.take_answers().into_iter().find(|record| {
		let expected_txt = RData::TXT(TXT::new(vec![domain_id.to_string()]));
		record.rdata() == &expected_txt
	});

	handle.abort();
	drop(handle);

	if response.is_some() {
		create_certificates_of_managed_urls_for_domain(
			connection,
			workspace_id,
			domain_id,
			domain_name,
			config,
		)
		.await?;

		db::update_workspace_domain_status(connection, domain_id, true).await?;

		return Ok(true);
	}

	Ok(false)
}

pub async fn create_certificates_of_managed_urls_for_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	domain_id: &Uuid,
	domain_name: &str,
	config: &Settings,
) -> Result<(), Error> {
	let managed_urls =
		db::get_all_managed_urls_for_domain(connection, domain_id).await?;

	for managed_url in managed_urls {
		infrastructure::create_certificates(
			workspace_id,
			&format!("certificate-{}", managed_url.id),
			&format!("tls-{}", managed_url.id),
			vec![format!("{}.{}", managed_url.sub_domain, domain_name)],
			config,
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
