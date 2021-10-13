use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use hex::ToHex;
use serde_json::{json, Map};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::{
		db_mapping::PreferredRecoveryOption,
		error::{id as ErrorId, message as ErrorMessage},
		rbac::{self, permissions, GOD_USER_ID},
		RegistryToken,
		RegistryTokenAccess,
	},
	pin_fn,
	service,
	utils::{
		constants::{request_keys, ResourceOwnerType},
		get_current_time,
		validator,
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
/// api including the database connections.
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

	app.post(
		"/sign-in",
		[EveMiddleware::CustomFunction(pin_fn!(sign_in))],
	);
	app.post(
		"/sign-up",
		[EveMiddleware::CustomFunction(pin_fn!(sign_up))],
	);
	app.post(
		"/sign-out",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(sign_out)),
		],
	);
	app.post("/join", [EveMiddleware::CustomFunction(pin_fn!(join))]);
	app.get(
		"/access-token",
		[EveMiddleware::CustomFunction(pin_fn!(get_access_token))],
	);
	app.get(
		"/email-valid",
		[EveMiddleware::CustomFunction(pin_fn!(is_email_valid))],
	);
	app.get(
		"/username-valid",
		[EveMiddleware::CustomFunction(pin_fn!(is_username_valid))],
	);
	app.post(
		"/forgot-password",
		[EveMiddleware::CustomFunction(pin_fn!(forgot_password))],
	);
	app.post(
		"/reset-password",
		[EveMiddleware::CustomFunction(pin_fn!(reset_password))],
	);
	app.post(
		"/docker-registry-token",
		[EveMiddleware::CustomFunction(pin_fn!(
			docker_registry_token_endpoint
		))],
	);
	app.get(
		"/docker-registry-token",
		[EveMiddleware::CustomFunction(pin_fn!(
			docker_registry_token_endpoint
		))],
	);

	app.post(
		"/list-recovery-options",
		[EveMiddleware::CustomFunction(pin_fn!(
			list_recovery_options
		))],
	);

	app
}

/// # Description
/// This function will enable the user to sign in into the api
/// required inputs:
/// ```
/// {
///    userId: username or email address
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
///    accessToken:
///    RefreshToken:
///    LoginId:
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn sign_in(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let user_id = body
		.get(request_keys::USER_ID)
		.map(|param| param.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?
		.to_lowercase();

	let password = body
		.get(request_keys::PASSWORD)
		.map(|param| param.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let user_data = db::get_user_by_username_email_or_phone_number(
		context.get_database_connection(),
		&user_id,
	)
	.await?
	.status(200)
	.body(error!(USER_NOT_FOUND).to_string())?;

	let success = service::validate_hash(password, &user_data.password)?;

	if !success {
		context.json(error!(INVALID_PASSWORD));
		return Ok(context);
	}

	let config = context.get_state().config.clone();
	let (jwt, login_id, refresh_token) = service::sign_in_user(
		context.get_database_connection(),
		&user_data.id,
		&config,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ACCESS_TOKEN: jwt,
		request_keys::REFRESH_TOKEN: refresh_token.to_simple().to_string().to_lowercase(),
		request_keys::LOGIN_ID: login_id.to_simple().to_string().to_lowercase(),
	}));
	Ok(context)
}

/// # Description
/// This function is used to register the user
/// required inputs:
/// Personal account:
/// ```
/// {
///    username:
///    password:
///    accountType: personal
///    firstName:
///    lastName:
///    backupEmail:
///    backupPhoneCountryCode:
///    backupPhoneNumber:
/// }
/// ```
/// Organisation account:
/// ```
/// {
///    username:
///    password:
///    accountType: personal
///    firstName:
///    lastName:
///    backupEmail:
///    backupPhoneCountryCode:
///    backupPhoneNumber:
///    organisationEmailLocal:
///    domain:
///    organisationName:
/// }
/// ```
///
/// In above paramters the user is only allowed to add email or phone number as
/// a back up. Both of them cannot be supplied
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
async fn sign_up(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let username = body
		.get(request_keys::USERNAME)
		.map(|param| param.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?
		.to_lowercase();

	let password = body
		.get(request_keys::PASSWORD)
		.map(|param| param.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let account_type = body
		.get(request_keys::ACCOUNT_TYPE)
		.map(|param| param.as_str())
		.flatten()
		.map(|a| a.parse::<ResourceOwnerType>().ok())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let first_name = body
		.get(request_keys::FIRST_NAME)
		.map(|param| param.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let last_name = body
		.get(request_keys::LAST_NAME)
		.map(|param| param.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	// store email as lowercase
	let backup_email = body
		.get(request_keys::BACKUP_EMAIL)
		.map(|param| {
			param
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?
		.map(|val| val.to_lowercase());

	// store Phone Country Code as uppercase
	let backup_phone_country_code = body
		.get(request_keys::BACKUP_PHONE_COUNTRY_CODE)
		.map(|param| {
			param
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?
		.map(|val| val.to_uppercase());

	let backup_phone_number = body
		.get(request_keys::BACKUP_PHONE_NUMBER)
		.map(|param| {
			param
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	// store org email as lower case
	let org_email_local = body
		.get(request_keys::ORGANISATION_EMAIL_LOCAL)
		.map(|param| {
			param
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?
		.map(|val| val.to_lowercase());

	// store org domain name as lower case
	let org_domain_name = body
		.get(request_keys::DOMAIN)
		.map(|param| {
			param
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?
		.map(|val| val.to_lowercase());

	let organisation_name = body
		.get(request_keys::ORGANISATION_NAME)
		.map(|param| {
			param
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?
		.map(|val| val.to_lowercase());

	let (user_to_sign_up, otp) = service::create_user_join_request(
		context.get_database_connection(),
		&username,
		account_type,
		password,
		(first_name, last_name),
		backup_email.as_deref(),
		backup_phone_country_code.as_deref(),
		backup_phone_number,
		org_email_local.as_deref(),
		org_domain_name.as_deref(),
		organisation_name.as_deref(),
	)
	.await?;
	// send otp
	service::send_user_sign_up_otp(
		context.get_database_connection(),
		user_to_sign_up,
		&otp,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

/// # Description
/// This function is used to sign-out the user, there will be an otp sent to
/// user's backup email or phone number
/// required inputs:
/// example: Authorization: <insert authToken>
/// auth token in the authorization headers
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
async fn sign_out(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let login_id =
		hex::decode(context.get_token_data().unwrap().login_id.clone())
			.status(400)
			.body(error!(UNAUTHORIZED).to_string())?;
	let user_id = context.get_token_data().unwrap().user.id.clone();

	db::get_user_login_for_user(
		context.get_database_connection(),
		&login_id,
		&user_id,
	)
	.await?
	.status(200)
	.body(error!(TOKEN_NOT_FOUND).to_string())?;

	db::delete_user_login_by_id(
		context.get_database_connection(),
		&login_id,
		&user_id,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS:true,
	}));
	Ok(context)
}

/// # Description
/// this function is used to verify the user's registration and register the
/// user
/// required inputs:
/// ```
/// {
///    verificationToken:
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
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn join(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let otp = body
		.get(request_keys::VERIFICATION_TOKEN)
		.map(|param| param.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let username = body
		.get(request_keys::USERNAME)
		.map(|param| param.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?
		.to_lowercase();

	let config = context.get_state().config.clone();

	let join_user = service::join_user(
		context.get_database_connection(),
		&config,
		otp,
		&username,
	)
	.await?;

	service::send_sign_up_complete_notification(
		join_user.welcome_email_to,
		join_user.backup_email_to,
		join_user.backup_phone_number_to,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ACCESS_TOKEN: join_user.jwt,
		request_keys::REFRESH_TOKEN: join_user.refresh_token.to_simple().to_string().to_lowercase(),
		request_keys::LOGIN_ID: join_user.login_id.to_simple().to_string().to_lowercase(),
	}));
	Ok(context)
}

/// # Description
/// This function is used to get a new access token for a currently logged in
/// user
/// required inputs:
/// refresh token in authorization header
/// example: Authorization: <insert refreshToken>
/// ```
/// {
///    loginId: email or username
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
///    accessToken:
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_access_token(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let refresh_token = context
		.get_header("Authorization")
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let login_id = context
		.get_param(request_keys::LOGIN_ID)
		.map(|value| hex::decode(value).ok())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let config = context.get_state().config.clone();
	let user_login = service::get_user_login_for_login_id(
		context.get_database_connection(),
		&login_id,
	)
	.await?;
	let success =
		service::validate_hash(&refresh_token, &user_login.refresh_token)?;

	if !success {
		Error::as_result()
			.status(200)
			.body(error!(UNAUTHORIZED).to_string())?;
	}

	let access_token = service::generate_access_token(
		context.get_database_connection(),
		&config,
		&user_login,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ACCESS_TOKEN: access_token
	}));
	Ok(context)
}

/// # Description
/// this function is used to check if the email address is valid or not
/// required inputs:
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
///    success: true false
///    available: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn is_email_valid(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let query = context.get_request().get_query().clone();

	let email_address = query
		.get(request_keys::EMAIL)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?
		.to_lowercase();

	let allowed = service::is_email_allowed(
		context.get_database_connection(),
		&email_address,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::AVAILABLE: allowed
	}));
	Ok(context)
}

/// # Description
/// This function is used to check if the username id valid or not
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
///    success: true or false
///    available: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn is_username_valid(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let query = context.get_request().get_query().clone();

	let username = query
		.get(request_keys::USERNAME)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?
		.to_lowercase();

	let allowed = service::is_username_allowed(
		context.get_database_connection(),
		&username,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::AVAILABLE: allowed
	}));
	Ok(context)
}

/// # Description
/// This function is used to recover the user's account incase the user forgets
/// the password by sending a recovery link or otp to user's registered backup
/// email or phone number
/// required inputs:
/// ```
/// {
///    userId:
///    preferredRecoveryOption: BackupPhoneNumber or BackupEmail
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
async fn forgot_password(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let user_id = body
		.get(request_keys::USER_ID)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?
		.to_lowercase();

	let preferred_recovery_option = body
		.get(request_keys::PREFERRED_RECOVERY_OPTION)
		.map(|param| param.as_str())
		.flatten()
		.map(|a| a.parse::<PreferredRecoveryOption>().ok())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	// service function should take care of otp generation and delivering the
	// otp to the preferred recovery option
	service::forgot_password(
		context.get_database_connection(),
		&user_id,
		preferred_recovery_option,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

/// # Description
/// This function is used to reset the password of user
/// required inputs:
/// ```
/// {
///    password:
///    verificationToken:
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
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn reset_password(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let new_password = body
		.get(request_keys::PASSWORD)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let token = body
		.get(request_keys::VERIFICATION_TOKEN)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let user_id = body
		.get(request_keys::USER_ID)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?
		.to_lowercase();

	let user = db::get_user_by_username_email_or_phone_number(
		context.get_database_connection(),
		&user_id,
	)
	.await?
	.status(400)
	.body(error!(EMAIL_TOKEN_NOT_FOUND).to_string())?;

	service::reset_password(
		context.get_database_connection(),
		new_password,
		token,
		&user.id,
	)
	.await?;

	service::send_user_reset_password_notification(
		context.get_database_connection(),
		user,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

/// # Description
/// This function is used to authenticate and login into the docker registry
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// ```
/// {
///    scope:
///    client_id:
///    service:
///    offline_token:
/// }
/// ```
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    token:
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn docker_registry_token_endpoint(
	context: EveContext,
	next: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let query = context.get_request().get_query();

	if query.get(request_keys::SCOPE).is_some() {
		// Authenticating an existing login
		docker_registry_authenticate(context, next).await
	} else {
		// Logging in
		docker_registry_login(context, next).await
	}
}

/// # Description
/// This function is used to login into the docker registry
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// ```
/// {
///    client_id: ,
///    offline_token: ,
///    service:
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
///    token:
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn docker_registry_login(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let query = context.get_request().get_query().clone();
	let config = context.get_state().config.clone();

	let _client_id = query
		.get(request_keys::SNAKE_CASE_CLIENT_ID)
		.status(400)
		.body(
			json!({
				request_keys::ERRORS: [{
					request_keys::CODE: ErrorId::UNAUTHORIZED,
					request_keys::MESSAGE: ErrorMessage::INVALID_CLIENT_ID,
					request_keys::DETAIL: []
				}]
			})
			.to_string(),
		)?;

	let _offline_token = query
		.get(request_keys::SNAKE_CASE_OFFLINE_TOKEN)
		.map(|value| {
			value.parse::<bool>().status(400).body(
				json!({
					request_keys::ERRORS: [{
						request_keys::CODE: ErrorId::UNAUTHORIZED,
						request_keys::MESSAGE: ErrorMessage::INVALID_OFFLINE_TOKEN,
						request_keys::DETAIL: []
					}]
				})
				.to_string(),
			)
		})
		.status(400)
		.body(
			json!({
				request_keys::ERRORS: [{
					request_keys::CODE: ErrorId::UNAUTHORIZED,
					request_keys::MESSAGE: ErrorMessage::OFFLINE_TOKEN_NOT_FOUND,
					request_keys::DETAIL: []
				}]
			})
			.to_string(),
		)??;

	let service = query.get(request_keys::SERVICE).status(400).body(
		json!({
			request_keys::ERRORS: [{
				request_keys::CODE: ErrorId::UNAUTHORIZED,
				request_keys::MESSAGE: ErrorMessage::SERVICE_NOT_FOUND,
				request_keys::DETAIL: []
			}]
		})
		.to_string(),
	)?;

	if service != &config.docker_registry.service_name {
		Error::as_result().status(400).body(
			json!({
				request_keys::ERRORS: [{
					request_keys::CODE: ErrorId::UNAUTHORIZED,
					request_keys::MESSAGE: ErrorMessage::INVALID_SERVICE,
					request_keys::DETAIL: []
				}]
			})
			.to_string(),
		)?;
	}

	let authorization = context
		.get_header("Authorization")
		.map(|value| value.replace("Basic ", ""))
		.map(|value| {
			base64::decode(value)
				.ok()
				.map(|value| String::from_utf8(value).ok())
				.flatten()
				.status(400)
				.body(
					json!({
						request_keys::ERRORS: [{
							request_keys::CODE: ErrorId::UNAUTHORIZED,
							request_keys::MESSAGE: ErrorMessage::AUTHORIZATION_PARSE_ERROR,
							request_keys::DETAIL: []
						}]
					})
					.to_string(),
				)
		})
		.status(400)
		.body(
			json!({
				request_keys::ERRORS: [{
					request_keys::CODE: ErrorId::UNAUTHORIZED,
					request_keys::MESSAGE: ErrorMessage::AUTHORIZATION_NOT_FOUND,
					request_keys::DETAIL: []
				}]
			})
			.to_string(),
		)??;

	let mut splitter = authorization.split(':');
	let username = splitter.next().status(400).body(
		json!({
			request_keys::ERRORS: [{
				request_keys::CODE: ErrorId::UNAUTHORIZED,
				request_keys::MESSAGE: ErrorMessage::USERNAME_NOT_FOUND,
				request_keys::DETAIL: []
			}]
		})
		.to_string(),
	)?;
	let password = splitter.next().status(400).body(
		json!({
			request_keys::ERRORS: [{
				request_keys::CODE: ErrorId::UNAUTHORIZED,
				request_keys::MESSAGE: ErrorMessage::PASSWORD_NOT_FOUND,
				request_keys::DETAIL: []
			}]
		})
		.to_string(),
	)?;
	let user =
		db::get_user_by_username(context.get_database_connection(), username)
			.await?
			.status(401)
			.body(
				json!({
					request_keys::ERRORS: [{
						request_keys::CODE: ErrorId::UNAUTHORIZED,
						request_keys::MESSAGE: ErrorMessage::USER_NOT_FOUND,
						request_keys::DETAIL: []
					}]
				})
				.to_string(),
			)?;

	// TODO API token as password instead of password, for TFA.
	// This will happen once the API token is merged in
	let success = service::validate_hash(password, &user.password)?;

	if !success {
		Error::as_result().status(401).body(
			json!({
				request_keys::ERRORS: [{
					request_keys::CODE: ErrorId::UNAUTHORIZED,
					request_keys::MESSAGE: ErrorMessage::INVALID_PASSWORD,
					request_keys::DETAIL: []
				}]
			})
			.to_string(),
		)?;
	}

	let iat = get_current_time().as_secs();

	let token = RegistryToken::new(
		config.docker_registry.issuer.clone(),
		iat,
		username.to_string(),
		&config,
		vec![],
	)
	.to_string(
		config.docker_registry.private_key.as_ref(),
		config.docker_registry.public_key_der(),
	)?;

	context.json(json!({ request_keys::TOKEN: token }));
	Ok(context)
}

/// # Description
/// This function is used to authenticate the user for docker registry
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
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
///    token:
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn docker_registry_authenticate(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let query = context.get_request().get_query().clone();
	let config = context.get_state().config.clone();

	let authorization = context
		.get_header("Authorization")
		.map(|value| value.replace("Basic ", ""))
		.map(|value| {
			base64::decode(value)
				.ok()
				.map(|value| String::from_utf8(value).ok())
				.flatten()
				.status(400)
				.body(
					json!({
						request_keys::ERRORS: [{
							request_keys::CODE: ErrorId::UNAUTHORIZED,
							request_keys::MESSAGE: ErrorMessage::AUTHORIZATION_PARSE_ERROR,
							request_keys::DETAIL: []
						}]
					})
					.to_string(),
				)
		})
		.status(400)
		.body(
			json!({
				request_keys::ERRORS: [{
					request_keys::CODE: ErrorId::UNAUTHORIZED,
					request_keys::MESSAGE: ErrorMessage::AUTHORIZATION_NOT_FOUND,
					request_keys::DETAIL: []
				}]
			})
			.to_string(),
		)??;

	let mut splitter = authorization.split(':');
	let username = splitter.next().status(400).body(
		json!({
			request_keys::ERRORS: [{
				request_keys::CODE: ErrorId::UNAUTHORIZED,
				request_keys::MESSAGE: ErrorMessage::USERNAME_NOT_FOUND,
				request_keys::DETAIL: []
			}]
		})
		.to_string(),
	)?;
	let password = splitter.next().status(400).body(
		json!({
			request_keys::ERRORS: [{
				request_keys::CODE: ErrorId::UNAUTHORIZED,
				request_keys::MESSAGE: ErrorMessage::PASSWORD_NOT_FOUND,
				request_keys::DETAIL: []
			}]
		})
		.to_string(),
	)?;
	let user =
		db::get_user_by_username(context.get_database_connection(), username)
			.await?
			.status(401)
			.body(
				json!({
					request_keys::ERRORS: [{
						request_keys::CODE: ErrorId::UNAUTHORIZED,
						request_keys::MESSAGE: ErrorMessage::USER_NOT_FOUND,
						request_keys::DETAIL: []
					}]
				})
				.to_string(),
			)?;

	let god_user_id = rbac::GOD_USER_ID.get().unwrap();
	let god_user = db::get_user_by_user_id(
		context.get_database_connection(),
		god_user_id.as_bytes(),
	)
	.await?
	.unwrap();
	// check if user is GOD_USER then return the token
	if username == god_user.username {
		// return token.
		if RegistryToken::parse(
			password,
			context.get_state().config.docker_registry.public_key_der(),
		)
		.is_ok()
		{
			context.json(json!({ request_keys::TOKEN: password }));
			return Ok(context);
		}
	}

	let success = service::validate_hash(password, &user.password)?;

	if !success {
		Error::as_result().status(401).body(
			json!({
				request_keys::ERRORS: [{
					request_keys::CODE: ErrorId::UNAUTHORIZED,
					request_keys::MESSAGE: ErrorMessage::INVALID_PASSWORD,
					request_keys::DETAIL: []
				}]
			})
			.to_string(),
		)?;
	}

	let scope = query.get(request_keys::SCOPE).unwrap();
	let mut splitter = scope.split(':');
	let access_type = splitter.next().status(401).body(
		json!({
			request_keys::ERRORS: [{
				request_keys::CODE: ErrorId::INVALID_REQUEST,
				request_keys::MESSAGE: ErrorMessage::ACCESS_TYPE_NOT_PRESENT,
				request_keys::DETAIL: []
			}]
		})
		.to_string(),
	)?;

	// check if access type is repository
	if access_type != request_keys::REPOSITORY {
		Error::as_result().status(400).body(
			json!({
				request_keys::ERRORS: [{
					request_keys::CODE: ErrorId::INVALID_REQUEST,
					request_keys::MESSAGE: ErrorMessage::INVALID_ACCESS_TYPE,
					request_keys::DETAIL: []
				}]
			})
			.to_string(),
		)?;
	}

	let repo = splitter.next().status(400).body(
		json!({
			request_keys::ERRORS: [{
				request_keys::CODE: ErrorId::INVALID_REQUEST,
				request_keys::MESSAGE: ErrorMessage::REPOSITORY_NOT_PRESENT,
				request_keys::DETAIL: []
			}]
		})
		.to_string(),
	)?;

	let action = splitter.next().status(400).body(
		json!({
			request_keys::ERRORS: [{
				request_keys::CODE: ErrorId::INVALID_REQUEST,
				request_keys::MESSAGE: ErrorMessage::ACTION_NOT_PRESENT,
				request_keys::DETAIL: []
			}]
		})
		.to_string(),
	)?;

	let required_permissions = action
		.split(',')
		.filter_map(|permission| match permission {
			"push" | "tag" => {
				Some(permissions::organisation::docker_registry::PUSH)
			}
			"pull" => Some(permissions::organisation::docker_registry::PULL),
			_ => None,
		})
		.map(String::from)
		.collect::<Vec<_>>();

	let split_array = repo.split('/').map(String::from).collect::<Vec<_>>();
	// reject if split array size is not equal to 2
	if split_array.len() != 2 {
		Error::as_result().status(400).body(
			json!({
				request_keys::ERRORS: [{
					request_keys::CODE: ErrorId::INVALID_REQUEST,
					request_keys::MESSAGE: ErrorMessage::NO_ORGANISATION_OR_REPOSITORY,
					request_keys::DETAIL: []
				}]
			})
			.to_string(),
		)?;
	}

	let org_name = split_array.get(0).unwrap(); // get first index from the vector
	let repo_name = split_array.get(1).unwrap();

	// check if repo name is valid
	let is_repo_name_valid = validator::is_docker_repo_name_valid(repo_name);
	if !is_repo_name_valid {
		Error::as_result().status(400).body(
			json!({
				request_keys::ERRORS: [{
					request_keys::CODE: ErrorId::INVALID_REQUEST,
					request_keys::MESSAGE: ErrorMessage::INVALID_REPOSITORY_NAME,
					request_keys::DETAIL: []
				}]
			})
			.to_string(),
		)?;
	}
	let org = db::get_organisation_by_name(
		context.get_database_connection(),
		org_name,
	)
	.await?
	.status(400)
	.body(
		json!({
			request_keys::ERRORS: [{
				request_keys::CODE: ErrorId::INVALID_REQUEST,
				request_keys::MESSAGE: ErrorMessage::RESOURCE_DOES_NOT_EXIST,
				request_keys::DETAIL: []
			}]
		})
		.to_string(),
	)?;

	let repository = db::get_repository_by_name(
		context.get_database_connection(),
		repo_name,
		&org.id,
	)
	.await?
	// reject request if repository does not exist
	.status(400)
	.body(
		json!({
			request_keys::ERRORS: [{
				request_keys::CODE: ErrorId::INVALID_REQUEST,
				request_keys::MESSAGE: ErrorMessage::RESOURCE_DOES_NOT_EXIST,
				request_keys::DETAIL: []
			}]
		})
		.to_string(),
	)?;

	// get repo id inorder to get resource details
	let resource = db::get_resource_by_id(
		context.get_database_connection(),
		&repository.id,
	)
	.await?
	.status(500)
	.body(
		json!({
			request_keys::ERRORS: [{
				request_keys::CODE: ErrorId::SERVER_ERROR,
				request_keys::MESSAGE: ErrorMessage::SERVER_ERROR,
				request_keys::DETAIL: []
			}]
		})
		.to_string(),
	)?;

	if resource.owner_id != org.id {
		log::error!(
			"Resource owner_id is not the same as org id. This is illegal"
		);
		Error::as_result().status(500).body(
			json!({
				request_keys::ERRORS: [{
					request_keys::CODE: ErrorId::SERVER_ERROR,
					request_keys::MESSAGE: ErrorMessage::SERVER_ERROR,
					request_keys::DETAIL: []
				}]
			})
			.to_string(),
		)?;
	}

	let org_id = org.id.encode_hex::<String>();
	let god_user_id = GOD_USER_ID.get().unwrap().as_bytes();

	// get all org roles for the user using the id
	let user_id = &user.id;
	let user_roles = db::get_all_organisation_roles_for_user(
		context.get_database_connection(),
		&user.id,
	)
	.await?;

	let required_role_for_user = user_roles.get(&org_id);
	let mut approved_permissions = vec![];

	for permission in required_permissions {
		let allowed =
			if let Some(required_role_for_user) = required_role_for_user {
				let resource_type_allowed = {
					if let Some(permissions) = required_role_for_user
						.resource_types
						.get(&resource.resource_type_id)
					{
						permissions.contains(&permission.to_string())
					} else {
						false
					}
				};
				let resource_allowed = {
					if let Some(permissions) =
						required_role_for_user.resources.get(&resource.id)
					{
						permissions.contains(&permission.to_string())
					} else {
						false
					}
				};
				let is_super_admin = {
					required_role_for_user.is_super_admin || {
						user_id == god_user_id
					}
				};
				resource_type_allowed || resource_allowed || is_super_admin
			} else {
				user_id == god_user_id
			};
		if !allowed {
			continue;
		}

		match permission.as_str() {
			permissions::organisation::docker_registry::PUSH => {
				approved_permissions.push("push".to_string());
			}
			permissions::organisation::docker_registry::PULL => {
				approved_permissions.push("pull".to_string());
			}
			_ => {}
		}
	}

	let iat = get_current_time().as_secs();

	let token = RegistryToken::new(
		config.docker_registry.issuer.clone(),
		iat,
		username.to_string(),
		&config,
		vec![RegistryTokenAccess {
			r#type: access_type.to_string(),
			name: repo.to_string(),
			actions: approved_permissions,
		}],
	)
	.to_string(
		config.docker_registry.private_key.as_ref(),
		config.docker_registry.public_key_der(),
	)?;

	context.json(json!({ request_keys::TOKEN: token }));
	Ok(context)
}

/// # Description
/// This function is used to list the recovery options the user has given
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// {
///    userId: username or email
/// }
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// {
///    backupPhoneNumber:
///    backupEmail:
///    success: true or false
/// }
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn list_recovery_options(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();
	let mut response_map = Map::new();
	// get user id from the body
	let user_id = body
		.get(request_keys::USER_ID)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let user = db::get_user_by_username_email_or_phone_number(
		context.get_database_connection(),
		user_id,
	)
	.await?
	.status(404)
	.body(error!(USER_NOT_FOUND).to_string())?;

	// mask phone number
	if let Some(phone_number) = user.backup_phone_number {
		let phone_number = service::mask_phone_number(&phone_number);
		let country_code = db::get_phone_country_by_country_code(
			context.get_database_connection(),
			&user.backup_phone_country_code.unwrap(),
		)
		.await?
		.status(500)
		.body(error!(INVALID_PHONE_NUMBER).to_string())?;
		response_map.insert(
			request_keys::BACKUP_PHONE_NUMBER.to_string(),
			format!("+{}{}", country_code.phone_code, phone_number).into(),
		);
	}

	if let (Some(backup_email_local), Some(backup_email_domain_id)) =
		(user.backup_email_local, user.backup_email_domain_id)
	{
		let email = format!(
			"{}@{}",
			service::mask_email_local(&backup_email_local),
			db::get_personal_domain_by_id(
				context.get_database_connection(),
				&backup_email_domain_id
			)
			.await?
			.status(500)?
			.name
		);

		response_map
			.insert(request_keys::BACKUP_EMAIL.to_string(), email.into());
	}

	response_map.insert(request_keys::SUCCESS.to_string(), true.into());
	let response = serde_json::to_value(response_map)?;
	context.json(response);
	Ok(context)
}
