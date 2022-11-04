use std::collections::HashMap;

use api_models::{models::auth::PreferredRecoveryOption, utils::Uuid};
use eve_rs::AsError;

use crate::{
	db::{self, DomainPlan, StaticSitePlan, User, UserToSignUp},
	error,
	models::{
		billing::{
			DatabaseBill,
			DeploymentBill,
			DockerRepositoryBill,
			DomainBill,
			ManagedUrlBill,
			SecretsBill,
			StaticSiteBill,
		},
		ResourceType,
	},
	utils::Error,
	Database,
};

mod email;
mod sms;

/// # Description
/// This function is used to notify the user that their sign up has been
/// successfully completed. This will ideally introduce them to the platform and
/// give them details on what they can do
///
/// # Arguments
/// * `welcome_email` - an Option<String> containing either String which has
/// user's personal or business email to send a welcome notification to or
/// `None`
/// * `recovery_email` - an Option<String> containing either String which has
/// user's recovery email to send a verification email to or `None`
/// * `recovery_phone_number` - an Option<String> containing either String which
/// has user's recovery phone number to send a verification sms to or `None`
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
pub async fn send_sign_up_complete_notification(
	welcome_email: Option<String>,
	recovery_email: Option<String>,
	recovery_phone_number: Option<String>,
	username: &str,
) -> Result<(), Error> {
	if let Some(welcome_email) = welcome_email {
		email::send_sign_up_completed_email(welcome_email.parse()?, username)
			.await?;
	}
	if let Some(recovery_email) = recovery_email {
		email::send_recovery_registration_mail(
			recovery_email.parse()?,
			username,
			&recovery_email,
		)
		.await?;
	}
	if let Some(phone_number) = recovery_phone_number {
		sms::send_recovery_registration_sms(&phone_number).await?;
	}
	Ok(())
}

/// # Description
/// This function is used to send email to the user to verify
///
/// # Arguments
/// * `new_email` - an Option<String> containing either String which has
/// new email added by the user to which the otp will be sent or `None`
/// * `otp` - a string containing otp to be sent
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
pub async fn send_email_verification_otp(
	new_email: String,
	otp: &str,
	username: &str,
	recovery_email: &str,
) -> Result<(), Error> {
	email::send_email_verification_otp(
		new_email.parse()?,
		otp,
		username,
		recovery_email,
	)
	.await
}

/// # Description
/// This function is used to send otp to verify user's phone number
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `country_code` - a string containing 2 letter country code
/// * `phone_number` - a string containing phone number of user
/// * `otp` - a string containing otp to be sent for verification
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Transaction`]: Transaction
pub async fn send_phone_number_verification_otp(
	connection: &mut <Database as sqlx::Database>::Connection,
	country_code: &str,
	phone_number: &str,
	otp: &str,
) -> Result<(), Error> {
	sms::send_phone_number_verification_otp(
		connection,
		country_code,
		phone_number,
		otp,
	)
	.await
}

/// # Description
/// This function is used to send otp to the user for sign-up
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `user` - an object of type [`UserToSignUp`]
/// * `otp` - a string containing otp to be sent
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Transaction`]: Transaction
pub async fn send_user_sign_up_otp(
	connection: &mut <Database as sqlx::Database>::Connection,
	user: &UserToSignUp,
	otp: &str,
) -> Result<(), Error> {
	// chcek if email is given as a recovery option
	if let Some((recovery_email_domain_id, recovery_email_local)) = user
		.recovery_email_domain_id
		.as_ref()
		.zip(user.recovery_email_local.as_ref())
	{
		let email = get_user_email(
			connection,
			recovery_email_domain_id,
			recovery_email_local,
		)
		.await?;
		email::send_user_verification_otp(email.parse()?, &user.username, otp)
			.await
	} else if let Some((phone_country_code, phone_number)) = user
		.recovery_phone_country_code
		.as_ref()
		.zip(user.recovery_phone_number.as_ref())
	{
		// check if phone number is given as a recovery
		let phone_number =
			get_user_phone_number(connection, phone_country_code, phone_number)
				.await?;
		sms::send_user_verification_otp(&phone_number, otp).await
	} else {
		Err(Error::empty()
			.status(400)
			.body(error!(NO_RECOVERY_OPTIONS).to_string()))
	}
}

/// # Description
/// This function is used to notify the user on all the recovery options that
/// their password has been changed
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `user` - an object of type [`User`] containing all details of user
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Transaction`]: Transaction
pub async fn send_password_changed_notification(
	connection: &mut <Database as sqlx::Database>::Connection,
	user: User,
) -> Result<(), Error> {
	// check if email is given as a recovery option
	if let Some((recovery_email_domain_id, recovery_email_local)) =
		user.recovery_email_domain_id.zip(user.recovery_email_local)
	{
		let email = get_user_email(
			connection,
			&recovery_email_domain_id,
			&recovery_email_local,
		)
		.await?;
		email::send_password_changed_notification(
			email.parse()?,
			&user.username,
		)
		.await?;
	}
	// check if phone number is given as a recovery
	if let Some((phone_country_code, phone_number)) = user
		.recovery_phone_country_code
		.zip(user.recovery_phone_number)
	{
		let phone_number = get_user_phone_number(
			connection,
			&phone_country_code,
			&phone_number,
		)
		.await?;
		sms::send_password_changed_notification(&phone_number).await?;
	}
	Ok(())
}

/// # Description
/// This function is used to notify the user that their password has been reset
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `user` - an object of type [`User`] containing all details of user
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Transaction`]: Transaction
pub async fn send_user_reset_password_notification(
	connection: &mut <Database as sqlx::Database>::Connection,
	user: User,
) -> Result<(), Error> {
	if let Some((phone_country_code, phone_number)) = user
		.recovery_phone_country_code
		.zip(user.recovery_phone_number)
	{
		let phone_number = get_user_phone_number(
			connection,
			&phone_country_code,
			&phone_number,
		)
		.await?;
		sms::send_user_reset_password_notification(&phone_number).await?;
	}
	if let Some((recovery_email_domain_id, recovery_email_local)) =
		user.recovery_email_domain_id.zip(user.recovery_email_local)
	{
		let email = get_user_email(
			connection,
			&recovery_email_domain_id,
			&recovery_email_local,
		)
		.await?;
		email::send_user_reset_password_notification(
			email.parse()?,
			&user.username,
		)
		.await?;
	}
	Ok(())
}

/// # Description
/// This function is used to send otp incase the user forgets their password and
/// requests for a reset of their password
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `user` - an object of type [`User`] containing all details of user
/// * `recovery_option` - an object of type [`PreferredRecoveryOption`]
/// * `otp` - a string containing otp to be sent to user
///  
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Transaction`]: Transaction
pub async fn send_forgot_password_otp(
	connection: &mut <Database as sqlx::Database>::Connection,
	user: User,
	recovery_option: &PreferredRecoveryOption,
	otp: &str,
) -> Result<(), Error> {
	// match on the recovery type
	match recovery_option {
		PreferredRecoveryOption::RecoveryEmail => {
			let email = get_user_email(
				connection,
				user.recovery_email_domain_id
					.as_ref()
					.status(400)
					.body(error!(WRONG_PARAMETERS).to_string())?,
				user.recovery_email_local
					.as_ref()
					.status(400)
					.body(error!(WRONG_PARAMETERS).to_string())?,
			)
			.await?;
			// send email
			email::send_forgot_password_otp(email.parse()?, otp, &user.username)
				.await
		}
		PreferredRecoveryOption::RecoveryPhoneNumber => {
			let phone_number = get_user_phone_number(
				connection,
				user.recovery_phone_country_code
					.as_ref()
					.status(400)
					.body(error!(WRONG_PARAMETERS).to_string())?,
				user.recovery_phone_number
					.as_ref()
					.status(400)
					.body(error!(WRONG_PARAMETERS).to_string())?,
			)
			.await?;
			// send SMS
			sms::send_forgot_password_otp(&phone_number, otp).await
		}
	}
}

/// # Description
/// This function is used to get the user's email address, from a local email
/// and a domain ID
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `domain_id` - An unsigned 8 bit integer array containing id of
/// workspace domain
/// * `email_string` - a string containing user's email_local
///  
/// # Returns
/// This function returns `Result<String, Error>` containing user's email
/// address or an error
///
/// [`Transaction`]: Transaction
async fn get_user_email(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
	email_string: &str,
) -> Result<String, Error> {
	let domain = db::get_personal_domain_by_id(connection, domain_id)
		.await?
		.status(500)?;
	let email = format!("{}@{}", email_string, domain.name);
	Ok(email)
}

/// # Description
/// This function is used to get the user's complete phone number, with the
/// country code
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `country_code` - a string containing 2 letter country code
/// * `phone_number` - a string containing user's phone number
///  
/// # Returns
/// This function returns `Result<String, Error>` containing user's complete
/// phone number or an error
///
/// [`Transaction`]: Transaction
async fn get_user_phone_number(
	connection: &mut <Database as sqlx::Database>::Connection,
	country_code: &str,
	phone_number: &str,
) -> Result<String, Error> {
	let country_code =
		db::get_phone_country_by_country_code(connection, country_code)
			.await?
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	let phone_number = format!("+{}{}", country_code.phone_code, phone_number);
	Ok(phone_number)
}

/// # Description
/// This function is used to notify the user that their resources has been
/// deleted because they haven't paid their bill
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `super_admin_id` - the ID of the user who owns the workspace
/// * `workspace_name` - the name of the workspace
/// * `month` - the month for which the bill wasn't paid
/// * `year` - the year for which the bill wasn't paid
/// * `total_bill` - the amount due by the user
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Transaction`]: Transaction
pub async fn send_unpaid_resources_deleted_email(
	connection: &mut <Database as sqlx::Database>::Connection,
	super_admin_id: Uuid,
	workspace_name: String,
	month_string: String,
	month_num: u32,
	year: i32,
	total_bill: f64,
) -> Result<(), Error> {
	let user = db::get_user_by_user_id(connection, &super_admin_id)
		.await?
		.status(500)?;

	let user_email = get_user_email(
		connection,
		user.recovery_email_domain_id
			.as_ref()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?,
		user.recovery_email_local
			.as_ref()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?,
	)
	.await?;

	email::send_unpaid_resources_deleted_email(
		user_email.parse()?,
		user.username,
		workspace_name,
		month_string,
		month_num,
		year,
		total_bill,
	)
	.await
}

/// # Description
/// This function is used to notify the user that their bill hasn't been paid
/// for and that their resources will be deleted soon if they don't pay
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `super_admin_id` - the ID of the user who owns the workspace
/// * `workspace_name` - the name of the workspace
/// * `month` - the month for which the bill wasn't paid
/// * `year` - the year for which the bill wasn't paid
/// * `total_bill` - the amount due by the user
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Transaction`]: Transaction
pub async fn send_bill_not_paid_reminder_email(
	connection: &mut <Database as sqlx::Database>::Connection,
	super_admin_id: Uuid,
	workspace_name: String,
	month_string: String,
	month_num: u32,
	year: i32,
	total_bill: f64,
) -> Result<(), Error> {
	let user = db::get_user_by_user_id(connection, &super_admin_id)
		.await?
		.status(500)?;

	let user_email = get_user_email(
		connection,
		user.recovery_email_domain_id
			.as_ref()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?,
		user.recovery_email_local
			.as_ref()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?,
	)
	.await?;

	email::send_bill_not_paid_reminder_email(
		user_email.parse()?,
		user.username,
		workspace_name,
		month_string,
		month_num,
		year,
		total_bill,
	)
	.await
}

/// # Description
/// This function is used to notify the user that their card was charged and the
/// payment failed.
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `super_admin_id` - the ID of the user who owns the workspace
/// * `workspace_name` - the name of the workspace
/// * `month` - the month for which the bill wasn't paid
/// * `year` - the year for which the bill wasn't paid
/// * `total_bill` - the amount due by the user
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Transaction`]: Transaction
pub async fn send_payment_failed_notification(
	connection: &mut <Database as sqlx::Database>::Connection,
	super_admin_id: Uuid,
	workspace_name: String,
	deployment_usages: &HashMap<Uuid, DeploymentBill>,
	database_usages: &HashMap<Uuid, DatabaseBill>,
	static_sites_usages: &HashMap<StaticSitePlan, StaticSiteBill>,
	managed_url_usages: &HashMap<u64, ManagedUrlBill>,
	docker_repository_usages: &[DockerRepositoryBill],
	domains_usages: &HashMap<DomainPlan, DomainBill>,
	secrets_usages: &HashMap<u64, SecretsBill>,
	month_string: String,
	month_num: u32,
	year: i32,
	total_bill: f64,
) -> Result<(), Error> {
	let user = db::get_user_by_user_id(connection, &super_admin_id)
		.await?
		.status(500)?;

	let user_email = get_user_email(
		connection,
		user.recovery_email_domain_id
			.as_ref()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?,
		user.recovery_email_local
			.as_ref()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?,
	)
	.await?;

	email::send_payment_failed_email(
		user_email.parse()?,
		user.username,
		workspace_name,
		deployment_usages.clone(),
		database_usages.clone(),
		static_sites_usages.clone(),
		managed_url_usages.clone(),
		docker_repository_usages.to_owned(),
		domains_usages.clone(),
		secrets_usages.clone(),
		month_string,
		month_num,
		year,
		total_bill,
	)
	.await
}

#[allow(clippy::too_many_arguments)]
pub async fn send_payment_success_notification(
	connection: &mut <Database as sqlx::Database>::Connection,
	super_admin_id: Uuid,
	workspace_name: String,
	deployment_usages: HashMap<Uuid, DeploymentBill>,
	database_usages: HashMap<Uuid, DatabaseBill>,
	static_sites_usages: HashMap<StaticSitePlan, StaticSiteBill>,
	managed_url_usages: HashMap<u64, ManagedUrlBill>,
	docker_repository_usages: Vec<DockerRepositoryBill>,
	domains_usages: HashMap<DomainPlan, DomainBill>,
	secrets_usages: HashMap<u64, SecretsBill>,
	month_string: String,
	month_num: u32,
	year: i32,
	total_bill: String,
	credit_amount: String,
	card_amount: String,
	credits_remaining: String,
	amount_paid: String,
) -> Result<(), Error> {
	let user = db::get_user_by_user_id(connection, &super_admin_id)
		.await?
		.status(500)?;

	let user_email = get_user_email(
		connection,
		user.recovery_email_domain_id
			.as_ref()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?,
		user.recovery_email_local
			.as_ref()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?,
	)
	.await?;

	email::send_payment_success_email(
		user_email.parse()?,
		user.username,
		workspace_name,
		deployment_usages,
		database_usages,
		static_sites_usages,
		managed_url_usages,
		docker_repository_usages,
		domains_usages,
		secrets_usages,
		month_string,
		month_num,
		year,
		total_bill,
		credit_amount,
		card_amount,
		credits_remaining,
		amount_paid,
	)
	.await
}

pub async fn resource_delete_action_email(
	connection: &mut <Database as sqlx::Database>::Connection,
	resource_name: &str,
	workspace_id: &Uuid,
	resource_type: &ResourceType,
	deleted_by_user_id: &Uuid,
) -> Result<(), Error> {
	let workspace = db::get_workspace_info(connection, workspace_id)
		.await?
		.status(500)?;

	let deleted_by = db::get_user_by_user_id(connection, deleted_by_user_id)
		.await?
		.status(500)?
		.first_name;

	// TODO -  change this and move to one function. Changes are done in PR #600
	let user = db::get_user_by_user_id(connection, &workspace.super_admin_id)
		.await?
		.status(500)?;

	let user_email = get_user_email(
		connection,
		user.recovery_email_domain_id
			.as_ref()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?,
		user.recovery_email_local
			.as_ref()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?,
	)
	.await?;

	// parsing personal-workspace-(uuid)
	let displayed_workspace_name = if let Some(Ok(user_id)) = workspace
		.name
		.strip_prefix("personal-workspace-")
		.map(Uuid::parse_str)
	{
		let user = db::get_user_by_user_id(connection, &user_id).await?;
		if let Some(user) = user {
			format!(
				"{} {}'s Personal Workspace",
				user.first_name, user.last_name
			)
		} else {
			workspace.name
		}
	} else {
		workspace.name
	};

	email::send_resource_deleted_email(
		displayed_workspace_name,
		resource_name.to_string(),
		user.first_name,
		resource_type.to_string(), /* converting to string as email template
		                            * does not support an enum as field in
		                            * it's struct */
		deleted_by,
		user_email.parse()?,
	)
	.await?;
	Ok(())
}

pub async fn domain_verification_email(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain: &str,
	workspace_id: &Uuid,
	domain_id: &Uuid,
	is_internal: bool,
	is_verified: bool,
) -> Result<(), Error> {
	let super_admin_id = db::get_workspace_info(connection, workspace_id)
		.await?
		.status(500)?
		.super_admin_id;

	// TODO -  change this and move to one function. Changes are done in PR #600
	let user = db::get_user_by_user_id(connection, &super_admin_id)
		.await?
		.status(500)?;

	let user_email = get_user_email(
		connection,
		user.recovery_email_domain_id
			.as_ref()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?,
		user.recovery_email_local
			.as_ref()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?,
	)
	.await?;

	if is_verified {
		email::send_domain_verified_email(
			domain.to_string(),
			user.first_name,
			domain_id.to_string(),
			user_email.parse()?,
		)
		.await?;
	} else {
		email::send_domain_unverified_email(
			domain.to_string(),
			user.first_name,
			is_internal,
			domain_id.to_string(),
			user_email.parse()?,
		)
		.await?;
	}
	Ok(())
}
