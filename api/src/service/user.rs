use api_models::{
	models::workspace::billing::{Address, PaymentMethod},
	utils::Uuid,
};
use eve_rs::AsError;
use reqwest::Client;

use crate::{
	db::{self, User},
	error,
	models::{PaymentIntentObject, PaymentMethodStatus, PaymentMethodUsage},
	service,
	utils::{get_current_time_millis, settings::Settings, validator, Error},
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
) -> Result<(), Error> {
	if !validator::is_email_valid(email_address) {
		Error::as_result()
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

	let otp = service::generate_new_otp();

	let token_expiry =
		get_current_time_millis() + service::get_join_token_expiry();
	let verification_token = service::hash(otp.as_bytes())?;

	service::send_email_verification_otp(email_address.to_string(), &otp)
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
		token_expiry,
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

	if email_verification_data.verification_token_expiry <
		get_current_time_millis()
	{
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
		db::delete_generic_domain(connection, &domain_id).await?;
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

	let token_expiry =
		get_current_time_millis() + service::get_join_token_expiry();
	let verification_token = service::hash(otp.as_bytes())?;

	db::add_phone_number_to_be_verified_for_user(
		connection,
		country_code,
		phone_number,
		user_id,
		&verification_token,
		token_expiry,
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

	if phone_verification_data.verification_token_expiry <
		get_current_time_millis()
	{
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

pub async fn update_billing_address(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	address_details: Address,
) -> Result<(), Error> {
	if address_details.address_line2.is_none() &&
		address_details.address_line3.is_some()
	{
		return Error::as_result()
			.status(400)
			.body(error!(ADDRESS_LINE_3_NOT_ALLOWED).to_string())?;
	}

	let workspace_data = db::get_workspace_info(connection, workspace_id)
		.await?
		.status(500)?;

	if let Some(address_id) = workspace_data.address_id {
		let address_details = &db::Address {
			id: address_id,
			first_name: address_details.first_name,
			last_name: address_details.last_name,
			address_line_1: address_details.address_line1,
			address_line_2: address_details.address_line2,
			address_line_3: address_details.address_line3,
			city: address_details.city,
			state: address_details.state,
			zip: address_details.zip,
			country: address_details.country,
		};

		db::update_billing_address(connection, address_details).await?;
	} else {
		return Error::as_result()
			.status(400)
			.body(error!(ADDRESS_NOT_FOUND).to_string())?;
	}

	Ok(())
}

pub async fn add_billing_address(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	address_details: Address,
) -> Result<(), Error> {
	if address_details.address_line2.is_none() &&
		address_details.address_line3.is_some()
	{
		return Error::as_result()
			.status(400)
			.body(error!(ADDRESS_LINE_3_NOT_ALLOWED).to_string())?;
	}

	if db::get_workspace_info(connection, workspace_id)
		.await?
		.status(500)?
		.address_id
		.is_some()
	{
		return Error::as_result()
			.status(400)
			.body(error!(ADDRESS_ALREADY_EXISTS).to_string())?;
	}

	let address_id = db::generate_new_address_id(connection).await?;

	let address_details = &db::Address {
		id: address_id.clone(),
		first_name: address_details.first_name,
		last_name: address_details.last_name,
		address_line_1: address_details.address_line1,
		address_line_2: address_details.address_line2,
		address_line_3: address_details.address_line3,
		city: address_details.city,
		state: address_details.state,
		zip: address_details.zip,
		country: address_details.country,
	};

	db::add_billing_address(connection, address_details).await?;

	db::add_billing_address_to_workspace(
		connection,
		&workspace_id,
		&address_id,
	)
	.await?;

	Ok(())
}

pub async fn get_card_details(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	config: &Settings,
) -> Result<Vec<PaymentMethod>, Error> {
	let payment_source_list =
		db::get_payment_methods_for_workspace(connection, workspace_id).await?;

	let mut cards = Vec::new();

	let client = Client::new();

	for payment_source in payment_source_list {
		let url = format!(
			"https://api.stripe.com/v1/payment_methods/{}",
			payment_source.id
		);

		let password: Option<String> = None;

		let card_details = client
			.get(&url)
			.basic_auth(&config.stripe.secret_key, password)
			.send()
			.await?
			.json::<PaymentMethod>()
			.await?;

		cards.push(card_details);
	}

	Ok(cards)
}

pub async fn add_card_details(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	config: &Settings,
) -> Result<PaymentIntentObject, Error> {
	let client = Client::new();

	let stripe_customer_id = db::get_workspace_info(connection, workspace_id)
		.await?
		.status(500)?
		.stripe_customer_id;

	let password: Option<String> = None;

	client
		.post("https://api.stripe.com/v1/setup_intents")
		.basic_auth(&config.stripe.secret_key, password)
		.query(&[
			("customer", stripe_customer_id.as_str()),
			// for now only accepting cards, other payment methods will be
			// accepted at later point of time
			("payment_method_types[]", "card"),
		])
		.send()
		.await?
		.json::<PaymentIntentObject>()
		.await
		.map_err(|e| e.into())
}
