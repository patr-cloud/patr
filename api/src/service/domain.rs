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
		validator,
		Error,
	},
	Database,
};

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
		// check if personal domain given by the user is registerd as a Org
		// domain
		if !is_domain_used_for_sign_up(connection, domain_name).await? {
			Error::as_result()
				.status(400)
				.body(error!(DOMAIN_BELONGS_TO_ORGANISATION).to_string())?;
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
			// check if personal domain given by the user is registerd as a Org
			// domain
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
		organisation_id,
		get_current_time_millis(),
	)
	.await?;
	db::create_generic_domain(
		connection,
		domain_id,
		domain_name,
		&ResourceOwnerType::Organisation,
	)
	.await?;
	db::add_to_organisation_domain(connection, domain_id).await?;

	Ok(domain_uuid)
}

// TODO make domain store the registrar and
// NS servers and auto configure accordingly too
pub async fn is_domain_verified(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &[u8],
) -> Result<bool, Error> {
	let domain = db::get_organisation_domain_by_id(connection, domain_id)
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

async fn is_domain_used_for_sign_up(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_name: &str,
) -> Result<bool, Error> {
	let org_domain_status =
		db::get_user_to_sign_up_by_org_domain_name(connection, domain_name)
			.await?;
	if let Some(org_domain_status) = org_domain_status {
		if org_domain_status.otp_expiry > get_current_time_millis() {
			return Ok(false);
		}
	}
	Ok(true)
}
