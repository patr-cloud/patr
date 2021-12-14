use cloudflare::{
	endpoints::zone::{
		CreateZone,
		CreateZoneParams,
		ListZones,
		ListZonesParams,
		Type,
	},
	framework::{
		async_api::{ApiClient, Client as CloudflareClient},
		auth::Credentials,
		Environment,
		HttpApiClientConfig,
	},
};
use eve_rs::AsError;
use hex::ToHex;
use tokio::{net::UdpSocket, task};
use trust_dns_client::{
	client::{AsyncClient, ClientHandle},
	rr::{DNSClass, Name, RData, RecordType},
	udp::UdpClientStream,
};
use uuid::Uuid;

use crate::{
	db,
	error,
	models::rbac,
	utils::{
		constants::ResourceOwnerType,
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
			Ok(Uuid::from_slice(domain.id.as_ref())?)
		}
	} else {
		// check if personal domain given by the user is registerd as a
		// workspace domain
		if !is_domain_used_for_sign_up(connection, domain_name).await? {
			Error::as_result()
				.status(400)
				.body(error!(DOMAIN_BELONGS_TO_WORKSPACE).to_string())?;
		}

		let domain_uuid = db::generate_new_domain_id(connection).await?;
		let domain_id = domain_uuid.as_bytes();
		db::create_generic_domain(
			connection,
			domain_id,
			domain_name,
			&ResourceOwnerType::Personal,
		)
		.await?;

		db::add_to_personal_domain(connection, domain_id).await?;

		Ok(domain_uuid)
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
	workspace_id: &[u8],
	is_patr_controled: bool,
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

	let domain_uuid = db::generate_new_domain_id(connection).await?;
	let domain_id = domain_uuid.as_bytes();
	db::create_resource(
		connection,
		domain_id,
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
		domain_id,
		domain_name,
		&ResourceOwnerType::Business,
	)
	.await?;
	db::add_to_workspace_domain(connection, domain_id, is_patr_controled)
		.await?;
	if is_patr_controled {
		// create zone

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

		// create zone
		client
			.request(&CreateZone {
				params: CreateZoneParams {
					name: &domain_name,
					jump_start: Some(false),
					account: &config.cloudflare.account_id,
					// Full because the DNS record
					zone_type: Some(Type::Full),
				},
			})
			.await?;

		let zone_identifier = client
			.request(&ListZones {
				params: ListZonesParams {
					name: Some(domain_name.to_string()),
					..Default::default()
				},
			})
			.await?
			.result
			.into_iter()
			.next()
			.status(500)?
			.id;

		// create a new function to store zone related data
		db::add_patr_controlled_domain(
			connection,
			domain_id,
			zone_identifier.as_bytes(),
		)
		.await?;
	}

	Ok(domain_uuid)
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
	domain_id: &[u8],
) -> Result<bool, Error> {
	let domain = db::get_workspace_domain_by_id(connection, domain_id)
		.await?
		.status(200)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let (mut client, bg) = AsyncClient::connect(
		UdpClientStream::<UdpSocket>::new("1.1.1.1:53".parse().unwrap()),
	)
	.await?;
	let handle = task::spawn(bg);
	let mut response = client
		.query(
			Name::from_utf8(format!("patrVerify.{}", domain.name)).unwrap(),
			DNSClass::IN,
			RecordType::CNAME,
		)
		.await?;
	let response = response.take_answers().into_iter().find(|record| {
		let expected_cname = RData::CNAME(
			Name::from_utf8(format!(
				"{}.patr.cloud",
				domain_id.encode_hex::<String>()
			))
			.unwrap(),
		);
		record.rdata() == &expected_cname
	});

	handle.abort();

	Ok(response.is_some())
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
