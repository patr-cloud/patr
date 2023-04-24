use api_models::{
	models::{
		user::{
			AddPersonalEmailRequest,
			AddPhoneNumberRequest,
			BasicUserInfo,
			ChangePasswordRequest,
			DeletePersonalEmailRequest,
			DeletePhoneNumberRequest,
			GetUserInfoByUserIdResponse,
			GetUserInfoResponse,
			ListPersonalEmailsResponse,
			ListPhoneNumbersResponse,
			ListUserWorkspacesResponse,
			SearchForUserRequest,
			SearchForUserResponse,
			UpdateRecoveryEmailRequest,
			UpdateRecoveryPhoneNumberRequest,
			UpdateUserInfoRequest,
			VerifyPersonalEmailRequest,
			VerifyPhoneNumberRequest,
		},
		workspace::Workspace,
	},
	utils::{DateTime, Uuid},
};
use axum::Router;
use chrono::{Datelike, Utc};

use crate::{db::User, prelude::*, utils::constants::request_keys};

mod api_token;
mod login;

/// This function is used to create a router for every endpoint in this file
pub fn create_sub_app(app: &App) -> Router<App> {
	Router::new()
		.mount_protected_dto(
			PlainTokenAuthenticator::new(),
			app.clone(),
			get_user_info,
		)
		.mount_protected_dto(
			PlainTokenAuthenticator::new(),
			app.clone(),
			update_user_info,
		)
		.mount_protected_dto(
			PlainTokenAuthenticator::new(),
			app.clone(),
			list_email_addresses,
		)
		.mount_protected_dto(
			PlainTokenAuthenticator::new().disallow_api_token(),
			app.clone(),
			add_email_address,
		)
		.mount_protected_dto(
			PlainTokenAuthenticator::new().disallow_api_token(),
			app.clone(),
			verify_email_address,
		)
		.mount_protected_dto(
			PlainTokenAuthenticator::new().disallow_api_token(),
			app.clone(),
			update_recovery_email_address,
		)
		.mount_protected_dto(
			PlainTokenAuthenticator::new().disallow_api_token(),
			app.clone(),
			delete_personal_email_address,
		)
		.mount_protected_dto(
			PlainTokenAuthenticator::new(),
			app.clone(),
			list_phone_numbers,
		)
		.mount_protected_dto(
			PlainTokenAuthenticator::new().disallow_api_token(),
			app.clone(),
			add_phone_number_for_user,
		)
		.mount_protected_dto(
			PlainTokenAuthenticator::new().disallow_api_token(),
			app.clone(),
			update_recovery_phone_number,
		)
		.mount_protected_dto(
			PlainTokenAuthenticator::new().disallow_api_token(),
			app.clone(),
			verify_phone_number,
		)
		.mount_protected_dto(
			PlainTokenAuthenticator::new().disallow_api_token(),
			app.clone(),
			delete_phone_number,
		)
		.mount_protected_dto(
			PlainTokenAuthenticator::new(),
			app.clone(),
			get_workspaces_for_user,
		)
		.mount_protected_dto(
			PlainTokenAuthenticator::new().disallow_api_token(),
			app.clone(),
			change_password,
		)
		.mount_protected_dto(
			PlainTokenAuthenticator::new(),
			app.clone(),
			get_user_info_by_user_id,
		)
		.mount_protected_dto(
			PlainTokenAuthenticator::new().disallow_api_token(),
			app.clone(),
			search_for_user,
		)
		.merge(login::create_sub_app(app))
		.merge(api_token::create_sub_app(app))
}

/// This function is used to get the user's information.
async fn get_user_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let user_id = context.get_token_data().unwrap().user_id().clone();
	let User {
		id,
		username,
		first_name,
		last_name,
		location,
		dob,
		bio,
		created,
		..
	} = db::get_user_by_user_id(context.get_database_connection(), &user_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let recovery_email = db::get_recovery_email_for_user(
		context.get_database_connection(),
		&user_id,
	)
	.await?;

	let secondary_emails = db::get_personal_emails_for_user(
		context.get_database_connection(),
		&user_id,
	)
	.await?
	.into_iter()
	.filter(|email| {
		if let Some(recovery_email) = &recovery_email {
			email != recovery_email
		} else {
			true
		}
	})
	.collect::<Vec<_>>();

	let recovery_phone_number = db::get_recovery_phone_number_for_user(
		context.get_database_connection(),
		&user_id,
	)
	.await?;

	let secondary_phone_numbers = db::get_phone_numbers_for_user(
		context.get_database_connection(),
		&user_id,
	)
	.await?
	.into_iter()
	.filter(|phone_number| {
		if let Some(recovery_phone_number) = &recovery_phone_number {
			phone_number != recovery_phone_number
		} else {
			true
		}
	})
	.collect::<Vec<_>>();

	context.success(GetUserInfoResponse {
		basic_user_info: BasicUserInfo {
			id,
			username,
			first_name,
			last_name,
			bio,
			location,
		},
		birthday: dob.map(DateTime),
		created: DateTime(created),
		recovery_email,
		secondary_emails,
		recovery_phone_number,
		secondary_phone_numbers,
	});
	Ok(context)
}

/// This function is used to get user info through userId
async fn get_user_info_by_user_id(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let user_id = context
		.get_param(request_keys::USER_ID)
		.and_then(|user_id_str| Uuid::parse_str(user_id_str.trim()).ok())
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let User {
		id,
		username,
		first_name,
		last_name,
		location,
		bio,
		..
	} = db::get_user_by_user_id(context.get_database_connection(), &user_id)
		.await?
		.status(400)
		.body(error!(PROFILE_NOT_FOUND).to_string())?;

	context.success(GetUserInfoByUserIdResponse {
		basic_user_info: BasicUserInfo {
			id,
			username,
			first_name,
			last_name,
			location,
			bio,
		},
	});
	Ok(context)
}

/// This function is used to update the user's information
async fn update_user_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let UpdateUserInfoRequest {
		first_name,
		last_name,
		birthday,
		bio,
		location,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let dob_string = birthday.as_ref().map(|value| value.to_string());

	// If no parameters to update
	first_name
		.as_ref()
		.or(last_name.as_ref())
		.or(dob_string.as_ref())
		.or(bio.as_ref())
		.or(location.as_ref())
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let user_id = context.get_token_data().unwrap().user_id().clone();

	if let Some(dob) = birthday.as_ref() {
		if (Utc::now().year() - dob.year()) < 13 {
			Error::as_result()
				.status(400)
				.body(error!(INVALID_BIRTHDAY).to_string())?;
		}
	}

	db::update_user_data(
		context.get_database_connection(),
		&user_id,
		first_name.as_deref(),
		last_name.as_deref(),
		birthday.map(|DateTime(bday)| bday).as_ref(),
		bio.as_deref(),
		location.as_deref(),
	)
	.await?;

	context.success(UpdateUserInfoResponse {});
	Ok(context)
}

/// This function is used to add a new email address
async fn add_email_address(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let AddPersonalEmailRequest { email } =
		context
			.get_body_as()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?;
	let email_address = email.to_lowercase();

	let config = context.get_state().config.clone();

	let user_id = context.get_token_data().unwrap().user_id().clone();

	service::add_personal_email_to_be_verified_for_user(
		context.get_database_connection(),
		&email_address,
		&user_id,
		&config,
	)
	.await?;

	context.success(AddPersonalEmailResponse {});
	Ok(context)
}

/// This function is used to list the email addresses registered with user
async fn list_email_addresses(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let user_id = context.get_token_data().unwrap().user_id().clone();

	let recovery_email = db::get_recovery_email_for_user(
		context.get_database_connection(),
		&user_id,
	)
	.await?;

	let secondary_emails = db::get_personal_emails_for_user(
		context.get_database_connection(),
		&user_id,
	)
	.await?
	.into_iter()
	.filter(|email| {
		if let Some(recovery_email) = &recovery_email {
			email != recovery_email
		} else {
			true
		}
	})
	.collect::<Vec<_>>();

	context.success(ListPersonalEmailsResponse {
		recovery_email,
		secondary_emails,
	});
	Ok(context)
}

/// This function is used to list the phone numbers registered with the user
async fn list_phone_numbers(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let user_id = context.get_token_data().unwrap().user_id().clone();

	let recovery_phone_number = db::get_recovery_phone_number_for_user(
		context.get_database_connection(),
		&user_id,
	)
	.await?;

	let secondary_phone_numbers = db::get_phone_numbers_for_user(
		context.get_database_connection(),
		&user_id,
	)
	.await?
	.into_iter()
	.filter(|phone_number| {
		if let Some(recovery_phone_number) = &recovery_phone_number {
			phone_number != recovery_phone_number
		} else {
			true
		}
	})
	.collect::<Vec<_>>();

	context.success(ListPhoneNumbersResponse {
		recovery_phone_number,
		secondary_phone_numbers,
	});
	Ok(context)
}

/// This function is used to update the back up email address of the user
async fn update_recovery_email_address(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let UpdateRecoveryEmailRequest { recovery_email } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let email_address = recovery_email.to_lowercase();

	let user_id = context.get_token_data().unwrap().user_id().clone();

	service::update_user_recovery_email(
		context.get_database_connection(),
		&user_id,
		&email_address,
	)
	.await?;

	context.success(UpdateRecoveryEmailResponse {});
	Ok(context)
}

/// This function is used to update the recovery phone number of the user
async fn update_recovery_phone_number(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let UpdateRecoveryPhoneNumberRequest {
		recovery_phone_country_code,
		recovery_phone_number: phone_number,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let country_code = recovery_phone_country_code.to_uppercase();

	let user_id = context.get_token_data().unwrap().user_id().clone();

	service::update_user_recovery_phone_number(
		context.get_database_connection(),
		&user_id,
		&country_code,
		&phone_number,
	)
	.await?;

	context.success(UpdateRecoveryPhoneNumberResponse {});
	Ok(context)
}

/// This function is used to delete a personal email address
async fn delete_personal_email_address(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let DeletePersonalEmailRequest { email } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let email_address = email.to_lowercase();

	let user_id = context.get_token_data().unwrap().user_id().clone();

	service::delete_personal_email_address(
		context.get_database_connection(),
		&user_id,
		&email_address,
	)
	.await?;

	context.success(DeletePersonalEmailResponse {});
	Ok(context)
}

/// This function is used to add phone number to the user's account
async fn add_phone_number_for_user(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let AddPhoneNumberRequest {
		country_code,
		phone_number,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	// two letter country code instead of the numeric one
	let country_code = country_code.to_uppercase();

	let user_id = context.get_token_data().unwrap().user_id().clone();

	let otp = service::add_phone_number_to_be_verified_for_user(
		context.get_database_connection(),
		&user_id,
		&country_code,
		&phone_number,
	)
	.await?;
	service::send_phone_number_verification_otp(
		context.get_database_connection(),
		&country_code,
		&phone_number,
		&otp,
	)
	.await?;

	context.success(AddPhoneNumberResponse {});
	Ok(context)
}

/// This function is used to verify user's phone number
async fn verify_phone_number(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let VerifyPhoneNumberRequest {
		country_code,
		phone_number,
		verification_token: otp,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	// two letter country code instead of the numeric one
	let country_code = country_code.to_uppercase();

	let user_id = context.get_token_data().unwrap().user_id().clone();

	service::verify_phone_number_for_user(
		context.get_database_connection(),
		&user_id,
		&country_code,
		&phone_number,
		&otp,
	)
	.await?;

	context.success(VerifyPhoneNumberResponse {});
	Ok(context)
}

/// This function is used to delete user's phone number
async fn delete_phone_number(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let DeletePhoneNumberRequest {
		country_code,
		phone_number,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	// two letter country code instead of the numeric one
	let country_code = country_code.to_uppercase();

	let user_id = context.get_token_data().unwrap().user_id().clone();

	service::delete_phone_number(
		context.get_database_connection(),
		&user_id,
		&country_code,
		&phone_number,
	)
	.await?;

	context.success(DeletePhoneNumberResponse {});
	Ok(context)
}

/// This function is used to verify user's email address
async fn verify_email_address(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let VerifyPersonalEmailRequest {
		email,
		verification_token: otp,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let email_address = email.to_lowercase();

	let user_id = context.get_token_data().unwrap().user_id().clone();

	service::verify_personal_email_address_for_user(
		context.get_database_connection(),
		&user_id,
		&email_address,
		&otp,
	)
	.await?;

	context.success(VerifyPersonalEmailResponse {});
	Ok(context)
}

/// This function is used to get a list of all workspaces which the user is a
/// member of
async fn get_workspaces_for_user(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let user_id = context.get_token_data().unwrap().user_id().clone();
	let workspaces = db::get_all_workspaces_for_user(
		context.get_database_connection(),
		&user_id,
	)
	.await?
	.into_iter()
	.map(|workspace| Workspace {
		id: workspace.id,
		name: workspace.name,
		active: workspace.active,
		super_admin_id: workspace.super_admin_id,
		alert_emails: workspace.alert_emails,
		default_payment_method_id: workspace.default_payment_method_id,
		is_verified: !workspace.is_spam,
	})
	.collect::<Vec<_>>();

	context.success(ListUserWorkspacesResponse { workspaces });
	Ok(context)
}

/// This function is used to change the password of user
async fn change_password(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let ChangePasswordRequest {
		current_password,
		new_password,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let user_id = context.get_token_data().unwrap().user_id().clone();

	let user = service::change_password_for_user(
		context.get_database_connection(),
		&user_id,
		&current_password,
		&new_password,
	)
	.await?;
	service::send_password_changed_notification(
		context.get_database_connection(),
		user,
	)
	.await?;

	context.success(ChangePasswordResponse {});
	Ok(context)
}

async fn search_for_user(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let SearchForUserRequest { query } = context
		.get_query_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	if query.is_empty() {
		return Error::as_result()
			.status(401)
			.body(error!(WRONG_PARAMETERS).to_string());
	}

	let users =
		db::search_for_users(context.get_database_connection(), &query).await?;

	context.success(SearchForUserResponse { users });
	Ok(context)
}
