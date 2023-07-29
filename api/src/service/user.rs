use std::collections::BTreeMap;

use ::redis::aio::MultiplexedConnection as RedisConnection;
use api_models::{
	models::workspace::{ResourcePermissionType, WorkspacePermission},
	utils::Uuid,
};
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

	let is_spam = spam_score.disposable || spam_score.fraud_score > 85;

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

	service::send_email_verification_otp(
		email_address.to_string(),
		&otp,
		&user.username,
		email_address,
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
	let is_password_same =
		service::validate_hash(new_password, &user.password)?;

	if !success && !user.password.is_empty() {
		Error::as_result()
			.status(400)
			.body(error!(INVALID_PASSWORD).to_string())?;
	}

	if is_password_same {
		Error::as_result()
			.status(400)
			.body(error!(PASSWORD_UNCHANGED).to_string())?;
	}

	// Check if the password passes standards
	if !validator::is_password_valid(new_password) {
		Error::as_result()
			.status(200)
			.body(error!(PASSWORD_TOO_WEAK).to_string())?;
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

	db::insert_raw_permissions_for_api_token(
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
		db::remove_all_permissions_for_api_token(connection, token_id).await?;
		db::insert_raw_permissions_for_api_token(
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

pub async fn get_derived_permissions_for_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	token_id: &Uuid,
) -> Result<BTreeMap<Uuid, WorkspacePermission>, Error> {
	let raw_token_permissions =
		db::get_raw_permissions_for_api_token(connection, token_id).await?;

	let user_id = db::get_active_user_api_token_by_id(connection, token_id)
		.await?
		.status(404)
		.body(error!(TOKEN_NOT_FOUND).to_string())?
		.user_id;

	let mut user_permissions =
		db::get_all_workspace_role_permissions_for_user(connection, &user_id)
			.await?;

	let mut derived_token_permissions = BTreeMap::new();

	for (workspace_id, token_workspace_permission) in raw_token_permissions {
		let Some(user_workspace_permission) =
			user_permissions.remove(&workspace_id)
		else {
			continue;
		};

		match (user_workspace_permission, token_workspace_permission) {
			(
				WorkspacePermission::SuperAdmin,
				WorkspacePermission::SuperAdmin,
			) => {
				derived_token_permissions
					.insert(workspace_id, WorkspacePermission::SuperAdmin);
			}
			(
				WorkspacePermission::SuperAdmin,
				WorkspacePermission::Member {
					permissions: member_permission,
				},
			) => {
				derived_token_permissions.insert(
					workspace_id,
					WorkspacePermission::Member {
						permissions: member_permission,
					},
				);
			}
			(
				WorkspacePermission::Member {
					permissions: mut user_member_permissions,
				},
				WorkspacePermission::Member {
					permissions: token_member_permissions,
				},
			) => {
				let mut derived_member_permissions = BTreeMap::new();
				for (permission_id, token_resource_permission_type) in
					token_member_permissions
				{
					let Some(user_resource_permission_type) =
						user_member_permissions.remove(&permission_id)
					else {
						continue;
					};

					match (
						user_resource_permission_type,
						token_resource_permission_type,
					) {
						(
							ResourcePermissionType::Include(user_includes),
							ResourcePermissionType::Include(token_includes),
						) => {
							derived_member_permissions.insert(
								permission_id,
								ResourcePermissionType::Include(
									token_includes
										.into_iter()
										.filter(|token_resource_id| {
											user_includes
												.contains(token_resource_id)
										})
										.collect(),
								),
							);
						}
						(
							ResourcePermissionType::Include(user_includes),
							ResourcePermissionType::Exclude(token_excludes),
						) => {
							derived_member_permissions.insert(
								permission_id,
								ResourcePermissionType::Include(
									user_includes
										.into_iter()
										.filter(|user_resource_id| {
											!token_excludes
												.contains(user_resource_id)
										})
										.collect(),
								),
							);
						}
						(
							ResourcePermissionType::Exclude(user_excludes),
							ResourcePermissionType::Include(token_include),
						) => {
							derived_member_permissions.insert(
								permission_id,
								ResourcePermissionType::Include(
									token_include
										.into_iter()
										.filter(|token_resource_id| {
											!user_excludes
												.contains(token_resource_id)
										})
										.collect(),
								),
							);
						}
						(
							ResourcePermissionType::Exclude(user_excludes),
							ResourcePermissionType::Exclude(token_excludes),
						) => {
							derived_member_permissions.insert(
								permission_id,
								ResourcePermissionType::Exclude(
									user_excludes
										.into_iter()
										.chain(token_excludes.into_iter())
										.collect(),
								),
							);
						}
					}
				}
				derived_token_permissions.insert(
					workspace_id,
					WorkspacePermission::Member {
						permissions: derived_member_permissions,
					},
				);
			}
			(
				WorkspacePermission::Member { .. },
				WorkspacePermission::SuperAdmin,
			) => {
				// ideally user being a member and his token as super_admin
				// won't be preset as it will be restricted by DB constraints
				// itself. So skipping these won't cause any problem
			}
		}
	}

	Ok(derived_token_permissions)
}
