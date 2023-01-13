use std::collections::BTreeMap;

use ::redis::aio::MultiplexedConnection as RedisConnection;
use api_models::{models::workspace::WorkspacePermission, utils::Uuid};
use chrono::{DateTime, Utc};
use eve_rs::AsError;
use reqwest::Client;
use sqlx::types::ipnetwork::IpNetwork;

use crate::{
	db::{self, User, UserLoginType},
	error,
	models::IpQualityScore,
	redis,
	service,
	utils::{settings::Settings, validator, Error},
	Database,
};

/// # Description
/// This functions adds personal email id to the email_address to be verified
/// table
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `email_address` - a string containing email address of user
/// * `user_id` - an unsigned 8 bit integer array containing id of user
///
/// # Returns
/// This function returns Result<(), Error> containing an empty response or an
/// error
///
/// [`Transaction`]: Transaction
pub async fn add_personal_email_to_be_verified_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	email_address: &str,
	user_id: &Uuid,
	config: &Settings,
) -> Result<(), Error> {
	if !validator::is_email_valid(email_address) {
		Error::as_result()
			.status(400)
			.body(error!(INVALID_EMAIL).to_string())?;
	}

	if email_address.contains('+') {
		log::trace!(
			"Invalid email address: {}, '+' not allowed in email address",
			email_address
		);

		return Error::as_result()
			.status(400)
			.body(error!(INVALID_EMAIL).to_string())?;
	}

	if db::get_user_by_email(connection, email_address)
		.await?
		.is_some()
	{
		Error::as_result()
			.status(400)
			.body(error!(EMAIL_TAKEN).to_string())?;
	}

	// Reference for APIs docs of ipqualityscore can be found in this
	// url https://www.ipqualityscore.com/documentation/email-validation/overview
	let spam_score = Client::new()
		.get(format!(
			"{}/{}/{}",
			config.ip_quality.host, config.ip_quality.token, email_address
		))
		.send()
		.await?
		.json::<IpQualityScore>()
		.await?;

	let is_spam = spam_score.disposable || spam_score.fraud_score > 75;

	if is_spam {
		log::warn!(
			"Disallowing adding email {} because it is found to be spam",
			email_address
		);
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	}

	let user = match db::get_user_by_user_id(connection, user_id).await? {
		Some(username) => username,
		None => {
			return Err(Error::empty()
				.status(500)
				.body(error!(SERVER_ERROR).to_string()))
		}
	};

	let otp = service::generate_new_otp();

	let token_expiry = Utc::now() + service::get_join_token_expiry();
	let verification_token = service::hash(otp.as_bytes())?;

	service::send_email_verification_otp(
		email_address.to_string(),
		&otp,
		&user.username,
		email_address,
	)
	.await?;

	// split email into 2 parts and get domain_id
	let (email_local, personal_domain_id) =
		service::split_email_with_domain_id(connection, email_address).await?;

	db::add_personal_email_to_be_verified_for_user(
		connection,
		&email_local,
		&personal_domain_id,
		user_id,
		&verification_token,
		&token_expiry,
	)
	.await?;

	Ok(())
}

/// # Description
/// This function is used to verify the email address of the user, by verifying
/// the otp
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `user_id` - an unsigned 8 bit integer array containing id of user
/// * `email_address` - a string containing email address of user
/// * `otp` - a string containing One-Time-Password
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Transaction`]: Transaction
pub async fn verify_personal_email_address_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	email_address: &str,
	otp: &str,
) -> Result<(), Error> {
	let email_verification_data =
		db::get_personal_email_to_be_verified_for_user(
			connection,
			user_id,
			email_address,
		)
		.await?
		.status(400)
		.body(error!(EMAIL_TOKEN_EXPIRED).to_string())?;

	let success = service::validate_hash(
		otp,
		&email_verification_data.verification_token_hash,
	)?;
	if !success {
		Error::as_result()
			.status(400)
			.body(error!(EMAIL_TOKEN_NOT_FOUND).to_string())?;
	}

	if email_verification_data.verification_token_expiry < Utc::now() {
		Error::as_result()
			.status(200)
			.body(error!(EMAIL_TOKEN_EXPIRED).to_string())?;
	}

	db::add_personal_email_for_user(
		connection,
		user_id,
		&email_verification_data.local,
		&email_verification_data.domain_id,
	)
	.await?;
	db::delete_personal_email_to_be_verified_for_user(
		connection,
		user_id,
		&email_verification_data.local,
		&email_verification_data.domain_id,
	)
	.await?;

	Ok(())
}

/// # Description
/// This function is used to change the password of the user
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `user_id` - an unsigned 8 bit integer array containing id of user
/// * `old_password` - a string containing previous password of the user
/// * `new_password` - a string containing new password of the user
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Transaction`]: Transaction
pub async fn change_password_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	current_password: &str,
	new_password: &str,
) -> Result<User, Error> {
	let user = db::get_user_by_user_id(connection, user_id)
		.await?
		.status(500)
		.body(error!(USER_NOT_FOUND).to_string())?;

	let success = service::validate_hash(current_password, &user.password)?;

	if !success {
		Error::as_result()
			.status(400)
			.body(error!(INVALID_PASSWORD).to_string())?;
	}

	let new_password = service::hash(new_password.as_bytes())?;

	db::update_user_password(connection, user_id, &new_password).await?;

	Ok(user)
}

pub async fn update_user_recovery_email(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	email_address: &str,
) -> Result<(), Error> {
	if !validator::is_email_valid(email_address) {
		Error::as_result()
			.status(200)
			.body(error!(INVALID_EMAIL).to_string())?;
	}
	// split email into 2 parts and get domain_id
	let (email_local, domain_id) =
		service::split_email_with_domain_id(connection, email_address).await?;

	// finally if everything checks out then change the personal email
	db::update_recovery_email_for_user(
		connection,
		user_id,
		&email_local,
		&domain_id,
	)
	.await
	.status(400)
	.body(error!(INVALID_EMAIL).to_string())?;

	Ok(())
}

pub async fn update_user_recovery_phone_number(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	country_code: &str,
	phone_number: &str,
) -> Result<(), Error> {
	if !validator::is_phone_number_valid(phone_number) {
		Error::as_result()
			.status(200)
			.body(error!(INVALID_PHONE_NUMBER).to_string())?;
	}

	let country_code =
		db::get_phone_country_by_country_code(connection, country_code)
			.await?
			.status(400)
			.body(error!(INVALID_COUNTRY_CODE).to_string())?;

	db::update_recovery_phone_number_for_user(
		connection,
		user_id,
		&country_code.country_code,
		phone_number,
	)
	.await
	.status(400)
	.body(error!(INVALID_PHONE_NUMBER).to_string())?;

	Ok(())
}

pub async fn delete_personal_email_address(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	email_address: &str,
) -> Result<(), Error> {
	let (email_local, domain_id) =
		service::split_email_with_domain_id(connection, email_address).await?;

	let user_data = db::get_user_by_user_id(connection, user_id).await?;

	let user_data = if let Some(user_data) = user_data {
		user_data
	} else {
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	};

	if let Some((recovery_email_local, recovery_domain_id)) = user_data
		.recovery_email_local
		.zip(user_data.recovery_email_domain_id)
	{
		if recovery_email_local == email_local &&
			recovery_domain_id == domain_id
		{
			return Error::as_result()
				.status(400)
				.body(error!(CANNOT_DELETE_RECOVERY_EMAIL).to_string())?;
		}
	}

	db::delete_personal_email_for_user(
		connection,
		user_id,
		&email_local,
		&domain_id,
	)
	.await?;

	let personal_email_count =
		db::get_personal_email_count_for_domain_id(connection, &domain_id)
			.await?;

	if personal_email_count == 0 {
		// first delete from personal domain
		db::delete_personal_domain(connection, &domain_id).await?;

		// then from the main domain table
		db::mark_domain_as_deleted(connection, &domain_id, &Utc::now()).await?;
	}

	Ok(())
}

pub async fn delete_phone_number(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	country_code: &str,
	phone_number: &str,
) -> Result<(), Error> {
	let user_data = db::get_user_by_user_id(connection, user_id).await?;

	let user_data = if let Some(user_data) = user_data {
		user_data
	} else {
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	};

	if let Some((recovery_country_code, recovery_phone_number)) = user_data
		.recovery_phone_country_code
		.zip(user_data.recovery_phone_number)
	{
		if recovery_country_code == country_code &&
			recovery_phone_number == phone_number
		{
			return Error::as_result().status(400).body(
				error!(CANNOT_DELETE_RECOVERY_PHONE_NUMBER).to_string(),
			)?;
		}
	}

	db::delete_phone_number_for_user(
		connection,
		user_id,
		country_code,
		phone_number,
	)
	.await?;

	Ok(())
}

pub async fn add_phone_number_to_be_verified_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	country_code: &str,
	phone_number: &str,
) -> Result<String, Error> {
	if !service::is_phone_number_allowed(connection, country_code, phone_number)
		.await?
	{
		Error::as_result()
			.status(400)
			.body(error!(PHONE_NUMBER_TAKEN).to_string())?;
	}

	let otp = service::generate_new_otp();

	let token_expiry = Utc::now() + service::get_join_token_expiry();
	let verification_token = service::hash(otp.as_bytes())?;

	db::add_phone_number_to_be_verified_for_user(
		connection,
		country_code,
		phone_number,
		user_id,
		&verification_token,
		&token_expiry,
	)
	.await?;

	Ok(otp)
}

pub async fn verify_phone_number_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	country_code: &str,
	phone_number: &str,
	otp: &str,
) -> Result<(), Error> {
	let phone_verification_data = db::get_phone_number_to_be_verified_for_user(
		connection,
		user_id,
		country_code,
		phone_number,
	)
	.await?
	.status(400)
	.body(error!(PHONE_NUMBER_TOKEN_EXPIRED).to_string())?;

	let success = service::validate_hash(
		otp,
		&phone_verification_data.verification_token_hash,
	)?;

	if phone_verification_data.verification_token_expiry < Utc::now() {
		Error::as_result()
			.status(200)
			.body(error!(PHONE_NUMBER_TOKEN_EXPIRED).to_string())?;
	}

	if !success {
		Error::as_result()
			.status(400)
			.body(error!(PHONE_NUMBER_TOKEN_NOT_FOUND).to_string())?;
	}

	db::add_phone_number_for_user(
		connection,
		user_id,
		&phone_verification_data.country_code,
		&phone_verification_data.phone_number,
	)
	.await?;
	db::delete_phone_number_to_be_verified_for_user(
		connection,
		user_id,
		&phone_verification_data.country_code,
		&phone_verification_data.phone_number,
	)
	.await?;

	Ok(())
}

pub async fn create_api_token_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	name: &str,
	permissions: &BTreeMap<Uuid, WorkspacePermission>,
	token_nbf: &Option<DateTime<Utc>>,
	token_exp: &Option<DateTime<Utc>>,
	allowed_ips: &Option<Vec<IpNetwork>>,
	request_id: &Uuid,
) -> Result<(Uuid, String), Error> {
	let token_id = db::generate_new_login_id(connection).await?;

	log::trace!(
		"request_id: {} creating api token for user: {} in database",
		request_id,
		user_id
	);

	let token = Uuid::new_v4().to_string();
	let user_facing_token = format!("patrv1.{}.{}", token, token_id);
	let token_hash = service::hash(token.as_bytes())?;
	let now = Utc::now();

	db::add_new_user_login(
		connection,
		&token_id,
		user_id,
		&UserLoginType::ApiToken,
		&now,
	)
	.await?;

	db::create_api_token_for_user(
		connection,
		&token_id,
		name,
		user_id,
		&token_hash,
		token_nbf.as_ref(),
		token_exp.as_ref(),
		allowed_ips.as_deref(),
		&now,
	)
	.await?;

	set_permissions_for_user_api_token(
		connection,
		&token_id,
		user_id,
		permissions,
	)
	.await?;

	Ok((token_id, user_facing_token))
}

pub async fn update_user_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	redis_connection: &mut RedisConnection,
	token_id: &Uuid,
	user_id: &Uuid,
	name: Option<&str>,
	token_nbf: Option<&DateTime<Utc>>,
	token_exp: Option<&DateTime<Utc>>,
	allowed_ips: Option<&[IpNetwork]>,
	permissions: Option<&BTreeMap<Uuid, WorkspacePermission>>,
) -> Result<(), Error> {
	let token = db::get_active_user_api_token_by_id(connection, token_id)
		.await?
		.filter(|token| &token.user_id == user_id)
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	if name.is_none() &&
		token_nbf.is_none() &&
		token_exp.is_none() &&
		allowed_ips.is_none() &&
		permissions.is_none()
	{
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	}

	db::update_user_api_token(
		connection,
		token_id,
		name,
		token_nbf,
		token_exp,
		allowed_ips,
	)
	.await?;

	if let Some(permissions) = permissions {
		db::remove_all_super_admin_permissions_for_api_token(
			connection, token_id,
		)
		.await?;
		db::remove_all_resource_type_permissions_for_api_token(
			connection, token_id,
		)
		.await?;
		db::remove_all_resource_permissions_for_api_token(connection, token_id)
			.await?;

		set_permissions_for_user_api_token(
			connection,
			token_id,
			&token.user_id,
			permissions,
		)
		.await?;
	}

	redis::delete_user_api_token_data(redis_connection, token_id).await?;

	Ok(())
}

async fn set_permissions_for_user_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	token_id: &Uuid,
	user_id: &Uuid,
	permissions: &BTreeMap<Uuid, WorkspacePermission>,
) -> Result<(), Error> {
	let user_permissions =
		db::get_all_workspace_role_permissions_for_user(connection, user_id)
			.await?;

	for (workspace_id, requested_permissions) in permissions {
		let workspace = db::get_workspace_info(connection, workspace_id)
			.await?
			.status(403)
			.body(error!(UNPRIVILEGED).to_string())?;
		let is_super_admin = &workspace.super_admin_id == user_id;
		if requested_permissions.is_super_admin {
			// Check if the workspace has this current user as the super admin
			if !is_super_admin {
				return Err(Error::empty()
					.status(403)
					.body(error!(UNPRIVILEGED).to_string()));
			}
			db::add_super_admin_permission_for_api_token(
				connection,
				token_id,
				workspace_id,
				user_id,
			)
			.await?;
		} else {
			// Check if the user has adequate permission on the given resource

			// Chech if the user has the resource type permissions that they're
			// requesting
			for (requested_resource_type_id, requested_permissions) in
				&requested_permissions.resource_type_permissions
			{
				// For every permission they're requesting on a resource type,
				// check against their allowed permissions
				for requested_permission_id in requested_permissions {
					let permission_allowed = is_super_admin ||
						user_permissions
							.get(workspace_id)
							.and_then(|permission| {
								permission
									.resource_type_permissions
									.get(requested_resource_type_id)
							})
							.map(|permissions| {
								permissions.contains(requested_permission_id)
							})
							.unwrap_or(false);
					if !permission_allowed {
						// That specific permission is not there for the
						// given resource_type
						return Err(Error::empty()
							.status(403)
							.body(error!(UNPRIVILEGED).to_string()));
					}

					// They have the permission. Add it
					db::add_resource_type_permission_for_api_token(
						connection,
						token_id,
						workspace_id,
						requested_resource_type_id,
						requested_permission_id,
					)
					.await?;
				}
			}

			// For every permission they're requesting on a resource, check
			// against their allowed permissions for either the resource or the
			// resource type
			for (requested_resource_id, requested_permissions) in
				&requested_permissions.resource_permissions
			{
				let requested_resource =
					db::get_resource_by_id(connection, requested_resource_id)
						.await?
						.status(404)
						.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

				// Check if they have the requested permissions on the given
				// resource type or on the given resource
				for requested_permission_id in requested_permissions {
					let permission_allowed = is_super_admin ||
						user_permissions
							.get(workspace_id)
							.and_then(|permission| {
								permission
									.resource_type_permissions
									.get(&requested_resource.resource_type_id)
							})
							.map(|permissions| {
								permissions.contains(requested_permission_id)
							})
							.unwrap_or(false) ||
						user_permissions
							.get(workspace_id)
							.and_then(|permission| {
								permission
									.resource_permissions
									.get(&requested_resource.id)
							})
							.map(|permissions| {
								permissions.contains(requested_permission_id)
							})
							.unwrap_or(false);

					if !permission_allowed {
						// That specific permission is not there for the
						// given resource_type or on the resource
						return Err(Error::empty()
							.status(403)
							.body(error!(UNPRIVILEGED).to_string()));
					}

					// They have the permission. Add it
					db::add_resource_permission_for_api_token(
						connection,
						token_id,
						workspace_id,
						requested_resource_id,
						requested_permission_id,
					)
					.await?;
				}
			}
		}
	}

	Ok(())
}

pub async fn get_permissions_for_user_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	token_id: &Uuid,
) -> Result<BTreeMap<Uuid, WorkspacePermission>, Error> {
	let mut permissions = BTreeMap::<_, WorkspacePermission>::new();

	for workspace_id in db::get_all_super_admin_workspace_ids_for_api_token(
		connection, token_id,
	)
	.await?
	{
		permissions.entry(workspace_id).or_default().is_super_admin = true;
	}

	for (workspace_id, resource_type_id, permission_id) in
		db::get_all_resource_type_permissions_for_api_token(
			connection, token_id,
		)
		.await?
	{
		permissions
			.entry(workspace_id)
			.or_default()
			.resource_type_permissions
			.entry(resource_type_id)
			.or_default()
			.insert(permission_id);
	}

	for (workspace_id, resource_id, permission_id) in
		db::get_all_resource_permissions_for_api_token(connection, token_id)
			.await?
	{
		permissions
			.entry(workspace_id)
			.or_default()
			.resource_permissions
			.entry(resource_id)
			.or_default()
			.insert(permission_id);
	}

	Ok(permissions)
}

pub async fn get_revalidated_permissions_for_user_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	token_id: &Uuid,
	user_id: &Uuid,
) -> Result<BTreeMap<Uuid, WorkspacePermission>, Error> {
	let existing_permissions =
		service::get_permissions_for_user_api_token(connection, token_id)
			.await?;
	let mut new_permissions = BTreeMap::<Uuid, WorkspacePermission>::new();

	for (workspace_id, permission) in &existing_permissions {
		let is_really_super_admin =
			db::get_workspace_info(connection, workspace_id)
				.await?
				.map(|workspace| &workspace.super_admin_id == user_id)
				.unwrap_or(false);

		if is_really_super_admin {
			// check super_admin_permission
			if permission.is_super_admin {
				new_permissions
					.entry(workspace_id.clone())
					.or_default()
					.is_super_admin = true;
				// If you have super admin permission, you don't really need
				// anything else
				continue;
			}

			// Since we know that the user is definitely a super admin
			// of this workspace. Don't bother checking any other
			// permissions. Just directly add them to the db
			for (resource_type_id, permissions) in
				&permission.resource_type_permissions
			{
				for permission_id in permissions {
					new_permissions
						.entry(workspace_id.clone())
						.or_default()
						.resource_type_permissions
						.entry(resource_type_id.clone())
						.or_default()
						.insert(permission_id.clone());
				}
			}
			for (resource_id, permissions) in &permission.resource_permissions {
				for permission_id in permissions {
					new_permissions
						.entry(workspace_id.clone())
						.or_default()
						.resource_permissions
						.entry(resource_id.clone())
						.or_default()
						.insert(permission_id.clone());
				}
			}
		} else {
			// The user is not a super admin of this workspace. Check their
			// actual permissions and only add the ones that they have the
			// permissions on

			let user_permissions =
				db::get_all_workspace_role_permissions_for_user(
					connection, user_id,
				)
				.await?;

			// Chech if the user has the resource type permissions that
			// they're requesting
			for (requested_resource_type_id, requested_permissions) in
				&permission.resource_type_permissions
			{
				// For every permission they're requesting on a resource
				// type, check against their allowed permissions
				for requested_permission_id in requested_permissions {
					let permission_allowed = user_permissions
						.get(workspace_id)
						.and_then(|permission| {
							permission
								.resource_type_permissions
								.get(requested_resource_type_id)
						})
						.map(|permissions| {
							permissions.contains(requested_permission_id)
						})
						.unwrap_or(false);
					if permission_allowed {
						// They have the permission. Add it
						new_permissions
							.entry(workspace_id.clone())
							.or_default()
							.resource_type_permissions
							.entry(requested_resource_type_id.clone())
							.or_default()
							.insert(requested_permission_id.clone());
					}

					// That specific permission is not there for the
					// given resource_type. Don't add it back
				}
			}

			// For every permission they're requesting on a resource, check
			// against their allowed permissions for either the resource or
			// the resource type
			for (requested_resource_id, requested_permissions) in
				&permission.resource_permissions
			{
				let requested_resource =
					db::get_resource_by_id(connection, requested_resource_id)
						.await?;
				let requested_resource =
					if let Some(resource) = requested_resource {
						resource
					} else {
						// If the resource doesn't exist anymore, don't add
						// the resource again.
						continue;
					};

				// Check if they have the requested permissions on the given
				// resource type or on the given resource
				for requested_permission_id in requested_permissions {
					let permission_allowed = user_permissions
						.get(workspace_id)
						.and_then(|permission| {
							permission
								.resource_type_permissions
								.get(&requested_resource.resource_type_id)
						})
						.map(|permissions| {
							permissions.contains(requested_permission_id)
						})
						.unwrap_or(false) ||
						user_permissions
							.get(workspace_id)
							.and_then(|permission| {
								permission
									.resource_permissions
									.get(&requested_resource.id)
							})
							.map(|permissions| {
								permissions.contains(requested_permission_id)
							})
							.unwrap_or(false);

					if permission_allowed {
						// They have the permission. Add it
						new_permissions
							.entry(workspace_id.clone())
							.or_default()
							.resource_permissions
							.entry(requested_resource_id.clone())
							.or_default()
							.insert(requested_permission_id.clone());
					}

					// That specific permission is not there for the
					// given resource. Don't add it back
				}
			}
		}
	}

	Ok(new_permissions)
}
