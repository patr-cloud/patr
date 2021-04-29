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
	utils::{validator, Error},
};

pub async fn add_domain_to_organisation(
	connection: &mut Transaction<'_, MySql>,
	domain_name: &str,
	organisation_id: &[u8],
	domain_type: &str,
) -> Result<Uuid, Error> {
	println!("TEST! in service/domain.rs");
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
	db::add_domain(connection, domain_id, &domain_name, domain_type).await?;

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
