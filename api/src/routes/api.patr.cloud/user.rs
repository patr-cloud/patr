use api_models::models::{
	user::{
		AddPersonalEmailRequest,
		AddPersonalEmailResponse,
		AddPhoneNumberRequest,
		AddPhoneNumberResponse,
		ChangePasswordRequest,
		ChangePasswordResponse,
		DeletePersonalEmailRequest,
		DeletePersonalEmailResponse,
		DeletePhoneNumberRequest,
		DeletePhoneNumberResponse,
		DeleteUserLoginResponse,
		GetUserInfoByUsernameResponse,
		GetUserInfoResponse,
		GetUserLoginInfoResponse,
		ListPersonalEmailsResponse,
		ListPhoneNumbersResponse,
		ListUserLoginsResponse,
		ListUserWorkspacesResponse,
		UpdateBackupEmailRequest,
		UpdateBackupEmailResponse,
		UpdateBackupPhoneNumberRequest,
		UpdateBackupPhoneNumberResponse,
		UpdateUserInfoRequest,
		UpdateUserInfoResponse,
		UserLogin,
		VerifyPersonalEmailRequest,
		VerifyPersonalEmailResponse,
		VerifyPhoneNumberRequest,
		VerifyPhoneNumberResponse,
	},
	workspace::Workspace,
};
use eve_rs::{App as EveApp, AsError, NextHandler};
use uuid::Uuid;

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::db_mapping::User,
	pin_fn,
	service,
	utils::{
		constants::request_keys,
		Error,
		ErrorData,
		EveContext,
		EveMiddleware,
	},
};

/// # Description
/// This function is used to create a sub app for every endpoint listed. It
/// creates an eve app which binds the endpoint with functions.
///
/// # Arguments
/// * `app` - an object of type [`App`] which contains all the configuration of
///   api including the
/// database connections.
///
/// # Returns
/// this function returns `EveApp<EveContext, EveMiddleware, App, ErrorData>`
/// containing context, middleware, object of [`App`] and Error
///
/// [`App`]: App
pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut app = create_eve_app(app);

	app.get(
		"/info",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(get_user_info)),
		],
	);
	app.post(
		"/info",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(update_user_info)),
		],
	);
	app.post(
		"/add-email-address",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(add_email_address)),
		],
	);
	app.get(
		"/list-email-address",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(list_email_addresses)),
		],
	);
	app.get(
		"/list-phone-numbers",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(list_phone_numbers)),
		],
	);
	app.post(
		"/update-backup-email",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(update_backup_email_address)),
		],
	);
	app.post(
		"/update-backup-phone",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(update_backup_phone_number)),
		],
	);
	app.post(
		"/add-phone-number",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(add_phone_number_for_user)),
		],
	);
	app.post(
		"/verify-phone-number",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(verify_phone_number)),
		],
	);
	app.delete(
		"/delete-personal-email",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(
				delete_personal_email_address
			)),
		],
	);
	app.delete(
		"/delete-phone-number",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(delete_phone_number)),
		],
	);
	app.post(
		"/verify-email-address",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(verify_email_address)),
		],
	);
	app.get(
		"/workspaces",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(get_workspaces_for_user)),
		],
	);
	app.post(
		"/change-password",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(change_password)),
		],
	);

	app.get(
		"/logins",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(get_all_logins_for_user)),
		],
	);

	app.get(
		"/logins/:loginId/info",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(get_login_info)),
		],
	);

	app.delete(
		"/logins/:loginId",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(delete_user_login)),
		],
	);
	app.get(
		"/:username/info",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(get_user_info_by_username)),
		],
	);
	app
}

/// # Description
/// This function is used to get the user's information.
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
///    username:
///    firstName:
///    lastName:
///    birthday:
///    bio:
///    location:
///    created:
///    emails:
///    phoneNumbers:
///    {
///       countryCode:
///       number:
///    }
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_user_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let user_id = context.get_token_data().unwrap().user.id.clone();
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

	let backup_email = db::get_backup_email_for_user(
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
		if let Some(backup_email) = &backup_email {
			email != backup_email
		} else {
			true
		}
	})
	.collect::<Vec<_>>();

	let backup_phone_number = db::get_backup_phone_number_for_user(
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
		if let Some(backup_phone_number) = &backup_phone_number {
			phone_number != backup_phone_number
		} else {
			true
		}
	})
	.collect::<Vec<_>>();

	context.success(GetUserInfoResponse {
		id,
		username,
		first_name,
		last_name,
		birthday: dob,
		bio,
		location,
		created,
		backup_email,
		secondary_emails,
		backup_phone_number,
		secondary_phone_numbers,
	});
	Ok(context)
}

/// # Description
/// This function is used to get user info through username
/// required inputs:
/// ```
/// {
///    username:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false,
///    username:,
///    firstName:,
///    lastName:,
///    birthday:,
///    bio:,
///    location:,
///    created:,
///    emails: [
///    ],
///    phoneNumbers: []
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_user_info_by_username(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let username = context
		.get_param(request_keys::USERNAME)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?
		.to_lowercase();

	let User {
		id,
		username,
		first_name,
		last_name,
		location,
		bio,
		..
	} = db::get_user_by_username(context.get_database_connection(), &username)
		.await?
		.status(400)
		.body(error!(PROFILE_NOT_FOUND).to_string())?;

	context.success(GetUserInfoByUsernameResponse {
		id,
		username,
		first_name,
		last_name,
		location,
		bio,
	});
	Ok(context)
}

/// # Description
/// This function is used to update the user's information
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// ```
/// {
///    firstName:
///    lastName:
///    dob:
///    bio:
///    location:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
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

	let dob_string = birthday.map(|value| value.to_string());

	// If no parameters to update
	first_name
		.as_ref()
		.or_else(|| last_name.as_ref())
		.or_else(|| dob_string.as_ref())
		.or_else(|| bio.as_ref())
		.or_else(|| location.as_ref())
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let user_id = context.get_token_data().unwrap().user.id.clone();

	db::update_user_data(
		context.get_database_connection(),
		&user_id,
		first_name.as_deref(),
		last_name.as_deref(),
		birthday,
		bio.as_deref(),
		location.as_deref(),
	)
	.await?;

	context.success(UpdateUserInfoResponse {});
	Ok(context)
}

/// # Description
/// This function is used to add a new email address
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// ```
/// {
///    email:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
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

	let user_id = context.get_token_data().unwrap().user.id.clone();

	service::add_personal_email_to_be_verified_for_user(
		context.get_database_connection(),
		&email_address,
		&user_id,
	)
	.await?;

	context.success(AddPersonalEmailResponse {});
	Ok(context)
}

/// # Description
/// This function is used to list the email addresses registered with user
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
///    emails: []
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn list_email_addresses(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let user_id = context.get_token_data().unwrap().user.id.clone();

	let backup_email = db::get_backup_email_for_user(
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
		if let Some(backup_email) = &backup_email {
			email != backup_email
		} else {
			true
		}
	})
	.collect::<Vec<_>>();

	context.success(ListPersonalEmailsResponse {
		backup_email,
		secondary_emails,
	});
	Ok(context)
}

/// # Description
/// This function is used to list the phone numbers registered with the user
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
///    phoneNumbers:
///    {
///       countryCode:
///       phoneNumber:
///    }
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn list_phone_numbers(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let user_id = context.get_token_data().unwrap().user.id.clone();

	let backup_phone_number = db::get_backup_phone_number_for_user(
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
		if let Some(backup_phone_number) = &backup_phone_number {
			phone_number != backup_phone_number
		} else {
			true
		}
	})
	.collect::<Vec<_>>();

	context.success(ListPhoneNumbersResponse {
		backup_phone_number,
		secondary_phone_numbers,
	});
	Ok(context)
}

/// # Description
/// This function is used to update the back up email address of the user
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// ```
/// {
///    backupEMail: new backupEmail
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn update_backup_email_address(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let UpdateBackupEmailRequest { backup_email } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let email_address = backup_email.to_lowercase();

	let user_id = context.get_token_data().unwrap().user.id.clone();

	service::update_user_backup_email(
		context.get_database_connection(),
		&user_id,
		&email_address,
	)
	.await?;

	context.success(UpdateBackupEmailResponse {});
	Ok(context)
}

/// # Description
/// This function is used to update the backup phone number of the user
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// ```
/// {
///    backupPhoneCountryCode:
///    backupPhoneNumber:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn update_backup_phone_number(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let UpdateBackupPhoneNumberRequest {
		backup_phone_country_code,
		backup_phone_number: phone_number,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let country_code = backup_phone_country_code.to_uppercase();

	let user_id = context.get_token_data().unwrap().user.id.clone();

	service::update_user_backup_phone_number(
		context.get_database_connection(),
		&user_id,
		&country_code,
		&phone_number,
	)
	.await?;

	context.success(UpdateBackupPhoneNumberResponse {});
	Ok(context)
}

/// # Description
/// This function is used to delete a personal email address
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// ```
/// {
///    email:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn delete_personal_email_address(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let DeletePersonalEmailRequest { email } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let email_address = email.to_lowercase();

	let user_id = context.get_token_data().unwrap().user.id.clone();

	service::delete_personal_email_address(
		context.get_database_connection(),
		&user_id,
		&email_address,
	)
	.await?;

	context.success(DeletePersonalEmailResponse {});
	Ok(context)
}

/// # Description
/// This function is used to add phone number to  the user's account
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// ```
/// {
///    countryCode:
///    phoneNumber:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
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

	let user_id = context.get_token_data().unwrap().user.id.clone();

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

/// # Description
/// This function is used to verify user's phone number
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// ```
/// {
///    countryCode:
///    phoneNumber:
///    verificationToken:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
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

	let user_id = context.get_token_data().unwrap().user.id.clone();

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

/// # Description
/// This function is used to delete user's phone number
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// ```
/// {
///    countryCode:
///    phoneNumber:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
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

	let user_id = context.get_token_data().unwrap().user.id.clone();

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

/// # Description
/// This function is used to verify user's email address
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// ```
/// {
///    email:
///    verificationToken:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
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

	let user_id = context.get_token_data().unwrap().user.id.clone();

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

/// # Description
/// This function is used to get a list of all workspaces in which the user
/// is a member
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false,
///    workspaces:
///    [
///       {
///           id: ,
///           name: ,
///           acitve: ,
///           created:         
///    
///       }
///    ]
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_workspaces_for_user(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let user_id = context.get_token_data().unwrap().user.id.clone();
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
	})
	.collect::<Vec<_>>();

	context.success(ListUserWorkspacesResponse { workspaces });
	Ok(context)
}

/// # Description
/// This function is used to change the password of user
/// required inputs:
/// auth token from headers
/// ```
/// {
///    newPassword:
///    password:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
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

	let user_id = context.get_token_data().unwrap().user.id.clone();

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

async fn get_all_logins_for_user(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let user_id = context.get_token_data().unwrap().user.id.clone();

	let logins = db::get_all_logins_for_user(
		context.get_database_connection(),
		&user_id,
	)
	.await?
	.into_iter()
	.map(|login| UserLogin {
		login_id: login.login_id,
		token_expiry: login.token_expiry,
		last_login: login.last_login,
		last_activity: login.last_activity,
	})
	.collect::<Vec<_>>();

	context.success(ListUserLoginsResponse { logins });
	Ok(context)
}

async fn get_login_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let login_id = context
		.get_param(request_keys::LOGIN_ID)
		.map(|param| Uuid::parse_str(param).ok())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let login =
		db::get_user_login(context.get_database_connection(), &login_id)
			.await?
			.map(|login| UserLogin {
				login_id: login.login_id,
				token_expiry: login.token_expiry,
				last_login: login.last_login,
				last_activity: login.last_activity,
			})
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?;

	context.success(GetUserLoginInfoResponse { login });
	Ok(context)
}

async fn delete_user_login(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let login_id = context
		.get_param(request_keys::LOGIN_ID)
		.map(|param| Uuid::parse_str(param).ok())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let user_id = context.get_token_data().unwrap().user.id.clone();

	db::delete_user_login_by_id(
		context.get_database_connection(),
		&login_id,
		&user_id,
	)
	.await?;

	context.success(DeleteUserLoginResponse {});
	Ok(context)
}
