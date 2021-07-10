use eve_rs::AsError;
use uuid::Uuid;

use crate::{
	db,
	error,
	models::rbac,
	utils::{get_current_time_millis, validator, Error},
	Database,
};

/// # Description
/// This function is used to check if the organisation name is valid 
/// or if it is already present in the database
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `organisation_name` - a string containing name of the organisation
///
/// # Returns
/// This function returns Result<bool, Error> containing bool which either
/// contains a boolean stating whether the organisation name is allowed or not
/// or an error
///
/// [`Transaction`]: Transaction
pub async fn is_organisation_name_allowed(
	connection: &mut <Database as sqlx::Database>::Connection,
	organisation_name: &str,
) -> Result<bool, Error> {
	if !validator::is_organisation_name_valid(&organisation_name) {
		Error::as_result()
			.status(200)
			.body(error!(INVALID_ORGANISATION_NAME).to_string())?;
	}

	let org =
		db::get_organisation_by_name(connection, organisation_name).await?;
	if org.is_some() {
		return Ok(false);
	}

	let org_sign_up_status = db::get_user_to_sign_up_by_organisation_name(
		connection,
		organisation_name,
	)
	.await?;

	if let Some(status) = org_sign_up_status {
		if status.otp_expiry > get_current_time_millis() {
			return Ok(false);
		}
	}
	Ok(true)
}

/// # Description
/// This function is used to create organisation
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `organisation_name` - a string containing name of the organisation
/// * `super_admin_id` - an unsigned 8 bit integer array containing id of the
/// super admin of
/// organisation
///
/// # Returns
/// This function returns `Result<Uuid, Error>` containing organisation id
/// (uuid) or an error
///
/// [`Transaction`]: Transaction
pub async fn create_organisation(
	connection: &mut <Database as sqlx::Database>::Connection,
	organisation_name: &str,
	super_admin_id: &[u8],
) -> Result<Uuid, Error> {
	if !is_organisation_name_allowed(connection, organisation_name).await? {
		Error::as_result()
			.status(400)
			.body(error!(ORGANISATION_EXISTS).to_string())?;
	}

	let organisation_id = db::generate_new_resource_id(connection).await?;
	let resource_id = organisation_id.as_bytes();

	db::begin_deferred_constraints(connection).await?;
	db::create_resource(
		connection,
		resource_id,
		&format!("Organisation: {}", organisation_name),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::ORGANISATION)
			.unwrap(),
		resource_id,
	)
	.await?;
	db::create_organisation(
		connection,
		resource_id,
		&organisation_name,
		super_admin_id,
		get_current_time_millis(),
	)
	.await?;
	db::end_deferred_constraints(connection).await?;

	Ok(organisation_id)
}

/// # Description
/// This function is used to convert username into personal organisation name
///
/// # Arguments
/// * `username` - a string containing username of the user
///
/// # Returns
/// This function returns a string containing the name of the personal
/// organisation
pub fn get_personal_org_name(username: &str) -> String {
	format!("personal-organisation-{}", username)
}
