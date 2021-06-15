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
	utils::{constants::ResourceOwnerType, validator, Error},
	Database,
};

/// # Description
/// This function checks if the personal domain exists, if it does not contain
/// domain this function will add the domain in the database and if the domain
/// is already present in organisation's table it will return an error
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
		if let ResourceOwnerType::Organisation = domain.r#type {
			Error::as_result()
				.status(500)
				.body(error!(DOMAIN_BELONGS_TO_ORGANISATION).to_string())
		} else {
			Ok(Uuid::from_slice(domain.id.as_ref())?)
		}
	} else {
		let domain_uuid = db::generate_new_domain_id(connection).await?;
		let domain_id = domain_uuid.as_bytes();
		db::create_generic_domain(
			connection,
			domain_id,
			&domain_name,
			&ResourceOwnerType::Personal,
		)
		.await?;
		db::add_to_personal_domain(connection, domain_id).await?;

		Ok(domain_uuid)
	}
}

/// # Description
/// This function adds the organisation domain into the database
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `domain_name` - A string which contains domain name of user's personal
///   email id
/// * `organisation_id` - An unsigned 8 bit integer array which contains id of
///   the organisation
///
/// # Returns
/// This function returns Result<Uuid, Error> containing uuid of domain uuid or
/// an error
///
///[`Transaction`]: Transaction
pub async fn add_domain_to_organisation(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_name: &str,
	organisation_id: &[u8],
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
			Error::as_result()
				.status(400)
				.body(error!(RESOURCE_EXISTS).to_string())?;
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
		organisation_id,
	)
	.await?;
	db::create_generic_domain(
		connection,
		domain_id,
		&domain_name,
		&ResourceOwnerType::Organisation,
	)
	.await?;
	db::add_to_organisation_domain(connection, domain_id).await?;

	Ok(domain_uuid)
}

/// # Description
/// This function checks if the domain is verified or not
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `domain_id` - An unsigned 8 bit integer array containing id of
///   organisation domain
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
	let domain = db::get_organisation_domain_by_id(connection, &domain_id)
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
			Name::from_utf8(format!("vicaraVerify.{}", domain.name)).unwrap(),
			DNSClass::IN,
			RecordType::CNAME,
		)
		.await?;
	let response = response.take_answers().into_iter().find(|record| {
		let expected_cname = RData::CNAME(
			Name::from_utf8(format!(
				"{}.vicara.co",
				domain_id.encode_hex::<String>()
			))
			.unwrap(),
		);
		record.rdata() == &expected_cname
	});

	handle.abort();

	Ok(response.is_some())
}
