use api_models::{
	models::prelude::*,
	utils::{DateTime, DecodedRequest, Paginated},
};
use axum::{extract::State, Extension, Router};
use chrono::{Datelike, Utc};

use crate::{db::User, models::UserAuthenticationData, prelude::*};

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
	mut connection: Connection,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: GetUserInfoPath,
		query: (),
		body,
	}: DecodedRequest<GetUserInfoRequest>,
) -> Result<GetUserInfoResponse, Error> {
	let user_id = token_data.user_id();
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
	} = db::get_user_by_user_id(&mut connection, &user_id)
		.await?
		.ok_or_else(|| ErrorType::internal_error())?;

	let recovery_email =
		db::get_recovery_email_for_user(&mut connection, &user_id).await?;

	let secondary_emails =
		db::get_personal_emails_for_user(&mut connection, &user_id)
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

	let recovery_phone_number =
		db::get_recovery_phone_number_for_user(&mut connection, &user_id)
			.await?;

	let secondary_phone_numbers =
		db::get_phone_numbers_for_user(&mut connection, &user_id)
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

	Ok(GetUserInfoResponse {
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
	})
}

/// This function is used to get user info through userId
async fn get_user_info_by_user_id(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: GetUserInfoByUserIdPath,
		query: (),
		body: GetUserInfoByUserIdRequest { user_id },
	}: DecodedRequest<GetUserInfoByUserIdRequest>,
) -> Result<GetUserInfoByUserIdResponse, Error> {
	let User {
		id,
		username,
		first_name,
		last_name,
		location,
		bio,
		..
	} = db::get_user_by_user_id(&mut connection, &user_id)
		.await?
		.ok_or_else(|| ErrorType::UserNotFound)?;

	Ok(GetUserInfoByUserIdResponse {
		basic_user_info: BasicUserInfo {
			id,
			username,
			first_name,
			last_name,
			location,
			bio,
		},
	})
}

/// This function is used to update the user's information
async fn update_user_info(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: UpdateUserInfoPath,
		query: (),
		body:
			UpdateUserInfoRequest {
				first_name,
				last_name,
				birthday,
				bio,
				location,
			},
	}: DecodedRequest<UpdateUserInfoRequest>,
) -> Result<(), Error> {
	let dob_string = birthday.as_ref().map(|value| value.to_string());

	// If no parameters to update
	first_name
		.as_ref()
		.or(last_name.as_ref())
		.or(dob_string.as_ref())
		.or(bio.as_ref())
		.or(location.as_ref())
		.ok_or_else(|| ErrorType::WrongParameters)?;

	let user_id = token_data.user_id();

	if let Some(dob) = birthday.as_ref() {
		if (Utc::now().year() - dob.year()) < 13 {
			return Err(ErrorType::InvalidBirthday.into());
		}
	}

	db::update_user_data(
		&mut connection,
		&user_id,
		first_name.as_deref(),
		last_name.as_deref(),
		birthday.map(|DateTime(bday)| bday).as_ref(),
		bio.as_deref(),
		location.as_deref(),
	)
	.await?;

	Ok(())
}

/// This function is used to add a new email address
async fn add_email_address(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: AddPersonalEmailPath,
		query: (),
		body: AddPersonalEmailRequest { email },
	}: DecodedRequest<AddPersonalEmailRequest>,
) -> Result<(), Error> {
	let email_address = email.to_lowercase();

	let user_id = token_data.user_id();

	service::add_personal_email_to_be_verified_for_user(
		&mut connection,
		&email_address,
		&user_id,
		&config,
	)
	.await?;

	Ok(())
}

/// This function is used to list the email addresses registered with user
async fn list_email_addresses(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: ListPersonalEmailsPath,
		query: (),
		body: (),
	}: DecodedRequest<ListPersonalEmailsRequest>,
) -> Result<ListPersonalEmailsResponse, Error> {
	let user_id = token_data.user_id();

	let recovery_email =
		db::get_recovery_email_for_user(&mut connection, &user_id).await?;

	let secondary_emails =
		db::get_personal_emails_for_user(&mut connection, &user_id)
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

	Ok(ListPersonalEmailsResponse {
		recovery_email,
		secondary_emails,
	})
}

/// This function is used to list the phone numbers registered with the user
async fn list_phone_numbers(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: ListPhoneNumbersPath,
		query: (),
		body: (),
	}: DecodedRequest<ListPhoneNumbersRequest>,
) -> Result<ListPhoneNumbersResponse, Error> {
	let user_id = token_data.user_id();

	let recovery_phone_number =
		db::get_recovery_phone_number_for_user(&mut connection, &user_id)
			.await?;

	let secondary_phone_numbers =
		db::get_phone_numbers_for_user(&mut connection, &user_id)
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

	Ok(ListPhoneNumbersResponse {
		recovery_phone_number,
		secondary_phone_numbers,
	})
}

/// This function is used to update the back up email address of the user
async fn update_recovery_email_address(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: UpdateRecoveryEmailPath,
		query: (),
		body: UpdateRecoveryEmailRequest { recovery_email },
	}: DecodedRequest<UpdateRecoveryEmailRequest>,
) -> Result<(), Error> {
	let email_address = recovery_email.to_lowercase();

	let user_id = token_data.user_id();

	service::update_user_recovery_email(
		&mut connection,
		&user_id,
		&email_address,
	)
	.await?;

	Ok(())
}

/// This function is used to update the recovery phone number of the user
async fn update_recovery_phone_number(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: UpdateRecoveryPhoneNumberPath,
		query: (),
		body:
			UpdateRecoveryPhoneNumberRequest {
				recovery_phone_country_code,
				recovery_phone_number,
			},
	}: DecodedRequest<UpdateRecoveryPhoneNumberRequest>,
) -> Result<(), Error> {
	let country_code = recovery_phone_country_code.to_uppercase();

	let user_id = token_data.user_id();

	service::update_user_recovery_phone_number(
		&mut connection,
		&user_id,
		&country_code,
		&recovery_phone_number,
	)
	.await?;

	Ok(())
}

/// This function is used to delete a personal email address
async fn delete_personal_email_address(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: DeletePersonalEmailPath,
		query: (),
		body: DeletePersonalEmailRequest { email },
	}: DecodedRequest<DeletePersonalEmailRequest>,
) -> Result<(), Error> {
	let email_address = email.to_lowercase();

	let user_id = token_data.user_id();

	service::delete_personal_email_address(
		&mut connection,
		&user_id,
		&email_address,
	)
	.await?;

	Ok(())
}

/// This function is used to add phone number to the user's account
async fn add_phone_number_for_user(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: AddPhoneNumberPath,
		query: (),
		body: AddPhoneNumberRequest {
			country_code,
			phone_number,
		},
	}: DecodedRequest<AddPhoneNumberRequest>,
) -> Result<(), Error> {
	// two letter country code instead of the numeric one
	let country_code = country_code.to_uppercase();

	let user_id = token_data.user_id();

	let otp = service::add_phone_number_to_be_verified_for_user(
		&mut connection,
		&user_id,
		&country_code,
		&phone_number,
	)
	.await?;
	service::send_phone_number_verification_otp(
		&mut connection,
		&country_code,
		&phone_number,
		&otp,
	)
	.await?;

	Ok(())
}

/// This function is used to verify user's phone number
async fn verify_phone_number(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: VerifyPhoneNumberPath,
		query: (),
		body:
			VerifyPhoneNumberRequest {
				country_code,
				phone_number,
				verification_token: otp,
			},
	}: DecodedRequest<VerifyPhoneNumberRequest>,
) -> Result<(), Error> {
	// two letter country code instead of the numeric one
	let country_code = country_code.to_uppercase();

	let user_id = token_data.user_id();

	service::verify_phone_number_for_user(
		&mut connection,
		&user_id,
		&country_code,
		&phone_number,
		&otp,
	)
	.await?;

	Ok(())
}

/// This function is used to delete user's phone number
async fn delete_phone_number(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: DeletePhoneNumberPath,
		query: (),
		body: DeletePhoneNumberRequest {
			country_code,
			phone_number,
		},
	}: DecodedRequest<DeletePhoneNumberRequest>,
) -> Result<(), Error> {
	// two letter country code instead of the numeric one
	let country_code = country_code.to_uppercase();

	let user_id = token_data.user_id();

	service::delete_phone_number(
		&mut connection,
		&user_id,
		&country_code,
		&phone_number,
	)
	.await?;

	Ok(())
}

/// This function is used to verify user's email address
async fn verify_email_address(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: VerifyPersonalEmailPath,
		query: (),
		body:
			VerifyPersonalEmailRequest {
				email,
				verification_token: otp,
			},
	}: DecodedRequest<VerifyPersonalEmailRequest>,
) -> Result<(), Error> {
	let email_address = email.to_lowercase();

	let user_id = token_data.user_id();

	service::verify_personal_email_address_for_user(
		&mut connection,
		&user_id,
		&email_address,
		&otp,
	)
	.await?;

	Ok(())
}

/// This function is used to get a list of all workspaces which the user is a
/// member of
async fn get_workspaces_for_user(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: ListUserWorkspacesPath,
		query: (),
		body: (),
	}: DecodedRequest<ListUserWorkspacesRequest>,
) -> Result<ListUserWorkspacesResponse, Error> {
	let user_id = token_data.user_id();
	let workspaces = db::get_all_workspaces_for_user(&mut connection, &user_id)
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

	Ok(ListUserWorkspacesResponse { workspaces })
}

/// This function is used to change the password of user
async fn change_password(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: ChangePasswordPath,
		query: (),
		body: ChangePasswordRequest {
			current_password,
			new_password,
		},
	}: DecodedRequest<ChangePasswordRequest>,
) -> Result<(), Error> {
	let user_id = token_data.user_id();

	let user = service::change_password_for_user(
		&mut connection,
		&user_id,
		&current_password,
		&new_password,
	)
	.await?;
	service::send_password_changed_notification(&mut connection, user).await?;

	Ok(())
}

async fn search_for_user(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: SearchForUserPath,
		query:
			Paginated {
				start: _,
				count: _,
				query: SearchForUserRequest { query },
			},
		body: (),
	}: DecodedRequest<SearchForUserRequest>,
) -> Result<SearchForUserResponse, Error> {
	if query.is_empty() {
		return Err(ErrorType::WrongParameters.into());
	}

	let users = db::search_for_users(&mut connection, &query).await?;

	Ok(SearchForUserResponse { users })
}
