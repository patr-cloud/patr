use eve_rs::AsError;
use sqlx::{MySql, Transaction};
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
	utils::{constants::AccountType, validator, Error},
};

pub async fn create_personal_domain(
	connection: &mut Transaction<'_, MySql>,
	domain_name: &str,
) -> Result<Vec<u8>, Error> {
	let domain_info = db::get_domain_by_name(connection, domain_name).await?;

	if domain_info.is_none() {
		let domain_uuid = db::generate_new_resource_id(connection).await?;
		let domain_id = domain_uuid.as_bytes();

		db::create_orphaned_resource(
			connection,
			domain_id,
			&format!("Domain: {}", domain_name),
			rbac::RESOURCE_TYPES
				.get()
				.unwrap()
				.get(rbac::resource_types::DOMAIN)
				.unwrap(),
		)
		.await?;

		db::add_to_generic_domain(
			connection,
			domain_id,
			&domain_name,
			AccountType::Personal,
		)
		.await?;

		db::add_to_personal_domain(
			connection,
			domain_id,
			AccountType::Personal,
		)
		.await?;

		Ok(domain_id.to_vec())
	} else {
		// i am unable to figure out how to convert from vec<u8> to uuid
		// so i made the function get_domain_by_name return vec<u8>
		let domain_uuid = domain_info.unwrap().id;
		Ok(domain_uuid)
	}
}

pub async fn add_domain_to_organisation(
	connection: &mut Transaction<'_, MySql>,
	domain_name: &str,
	organisation_id: &[u8],
) -> Result<Uuid, Error> {
	if !validator::is_domain_name_valid(domain_name).await {
		Error::as_result()
			.status(400)
			.body(error!(INVALID_DOMAIN_NAME).to_string())?;
	}

	if db::get_domain_by_name(connection, domain_name)
		.await?
		.is_some()
	{
		Error::as_result()
			.status(400)
			.body(error!(RESOURCE_EXISTS).to_string())?;
	}

	let domain_uuid = db::generate_new_resource_id(connection).await?;
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
	db::add_to_generic_domain(
		connection,
		domain_id,
		&domain_name,
		AccountType::Organisation,
	)
	.await?;

	db::add_to_organisation_domain(
		connection,
		domain_id,
		AccountType::Organisation,
		false,
	)
	.await?;

	Ok(domain_uuid)
}

// TODO make domain store the registrar and
// NS servers and auto configure accordingly too
pub async fn is_domain_verified(
	connection: &mut Transaction<'_, MySql>,
	domain_id: &[u8],
) -> Result<bool, Error> {
	let domain = db::get_domain_by_id(connection, &domain_id)
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
			Name::from_utf8(format!("{}.vicara.co", hex::encode(domain_id)))
				.unwrap(),
		);
		record.rdata() == &expected_cname
	});

	handle.abort();

	Ok(response.is_some())
}
