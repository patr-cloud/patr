use api_models::{
	models::{
		user::{
			AddPersonalEmailRequest,
			AddPersonalEmailResponse,
			AddPhoneNumberRequest,
			AddPhoneNumberResponse,
			BasicUserInfo,
			ChangePasswordRequest,
			ChangePasswordResponse,
			DeletePersonalEmailRequest,
			DeletePersonalEmailResponse,
			DeletePhoneNumberRequest,
			DeletePhoneNumberResponse,
			GetUserInfoByUserIdResponse,
			GetUserInfoResponse,
			ListPersonalEmailsResponse,
			ListPhoneNumbersResponse,
			ListUserWorkspacesResponse,
			SearchForUserRequest,
			SearchForUserResponse,
			UpdateRecoveryEmailRequest,
			UpdateRecoveryEmailResponse,
			UpdateRecoveryPhoneNumberRequest,
			UpdateRecoveryPhoneNumberResponse,
			UpdateUserInfoRequest,
			UpdateUserInfoResponse,
			VerifyPersonalEmailRequest,
			VerifyPersonalEmailResponse,
			VerifyPhoneNumberRequest,
			VerifyPhoneNumberResponse,
		},
		workspace::Workspace,
	},
	utils::{DateTime, Uuid},
};
use axum::{
	routing::{delete, get, post},
	Router,
};
use chrono::{Datelike, Utc};

use crate::{
	app::App,
	db::{self, User},
	error,
	service,
	utils::{constants::request_keys, Error},
};

mod api_token;
mod login;

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
pub async fn create_sub_route(app: &App) -> Router {
	let router = Router::new();

	// All middleware routes are PlainTokenAuthenticator routes
	router.route("/info", get(get_user_info));
	router.route("/info", post(update_user_info));
	router.route("/add-email-address", post(add_email_address));
	router.route("/list-email-address", get(list_email_addresses));
	router.route("/list-phone-numbers", get(list_phone_numbers));
	router.route(
		"/update-recovery-email",
		post(update_recovery_email_address),
	);
	router.route("/update-recovery-phone", post(update_recovery_phone_number));
	router.route("/add-phone-number", post(add_phone_number_for_user));
	router.route("/verify-phone-number", post(verify_phone_number));
	router.route(
		"/delete-personal-email",
		delete(delete_personal_email_address),
	);
	router.route("/delete-phone-number", delete(delete_phone_number));
	router.route("/verify-email-address", post(verify_email_address));
	router.route("/workspaces", get(get_workspaces_for_user));
	router.route("/change-password", post(change_password));

	router.route("/:userId/info", get(get_user_info_by_user_id));
	router.route("/search", get(search_for_user));

	router.nest("/", login::create_sub_route(app));
	router.nest("/", api_token::create_sub_route(app));

	router
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
async fn get_user_info(State(app): State<App>) -> Result<EveContext, Error> {
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

/// # Description
/// This function is used to get user info through userId
/// required inputs:
/// ```
/// {
///    userId:
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
async fn get_user_info_by_user_id(
	State(app): State<App>,
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
async fn update_user_info(State(app): State<App>) -> Result<EveContext, Error> {
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
	State(app): State<App>,
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
	State(app): State<App>,
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
	State(app): State<App>,
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

/// # Description
/// This function is used to update the back up email address of the user
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// ```
/// {
///    recoveryEMail: new recoveryEmail
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
async fn update_recovery_email_address(
	State(app): State<App>,
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

/// # Description
/// This function is used to update the recovery phone number of the user
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// ```
/// {
///    recoveryPhoneCountryCode:
///    recoveryPhoneNumber:
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
async fn update_recovery_phone_number(
	State(app): State<App>,
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
	State(app): State<App>,
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
	State(app): State<App>,
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
	State(app): State<App>,
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
	State(app): State<App>,
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
	State(app): State<App>,
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
	State(app): State<App>,
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
async fn change_password(State(app): State<App>) -> Result<EveContext, Error> {
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

async fn search_for_user(State(app): State<App>) -> Result<EveContext, Error> {
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
