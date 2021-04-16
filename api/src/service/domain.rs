use eve_rs::AsError;
use sqlx::{MySql, Transaction};
use uuid::Uuid;

use crate::{db, error, models::rbac, utils::Error};

pub async fn add_domain_to_organisation(
	connection: &mut Transaction<'_, MySql>,
	domain_name: &str,
	organisation_id: &[u8],
) -> Result<Uuid, Error> {
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
	db::add_domain(connection, domain_id, &domain_name).await?;

	Ok(domain_uuid)
}
