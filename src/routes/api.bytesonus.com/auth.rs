use crate::{
	app::{create_eve_app, App},
	db,
	models::{
		db_mapping::{UserEmailAddress, UserEmailAddressSignUp},
		error, AccessTokenData,
	},
	pin_fn,
	utils::{
		constants::request_keys, get_current_time, mailer, validator, EveContext, EveMiddleware,
	},
};

use argon2::Variant;
use async_std::task;
use eve_rs::{App as EveApp, Context, Error, NextHandler};
use rand::Rng;
use serde_json::{json, Value};
use uuid::Uuid;

pub fn create_sub_app(app: App) -> EveApp<EveContext, EveMiddleware, App> {
	let mut app = create_eve_app(app);

	app.post(
		"/sign-in",
		&[EveMiddleware::CustomFunction(pin_fn!(sign_in))],
	);
	app.post(
		"/sign-up",
		&[EveMiddleware::CustomFunction(pin_fn!(sign_up))],
	);
	app.post("/join", &[EveMiddleware::CustomFunction(pin_fn!(join))]);
	app.get(
		"/access-token",
		&[EveMiddleware::CustomFunction(pin_fn!(get_access_token))],
	);
	app.get(
		"/email-valid",
		&[EveMiddleware::CustomFunction(pin_fn!(is_email_valid))],
	);
	app.get(
		"/username-valid",
		&[EveMiddleware::CustomFunction(pin_fn!(is_username_valid))],
	);

	app
}

async fn sign_in(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let body = if let Some(body) = context.get_body_object() {
		body.clone()
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::WRONG_PARAMETERS,
			request_keys::MESSAGE: error::message::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let user_id = if let Some(Value::String(user_id)) = body.get(request_keys::USER_ID) {
		user_id
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::WRONG_PARAMETERS,
			request_keys::MESSAGE: error::message::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let password = if let Some(Value::String(password)) = body.get(request_keys::PASSWORD) {
		password
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::WRONG_PARAMETERS,
			request_keys::MESSAGE: error::message::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let user = if let Some(user) =
		db::get_user_by_username_or_email_or_phone_number(context.get_db_connection(), user_id)
			.await?
	{
		user
	} else {
		context.json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::USER_NOT_FOUND,
			request_keys::MESSAGE: error::message::USER_NOT_FOUND
		}));
		return Ok(context);
	};

	let success = argon2::verify_raw(
		password.as_bytes(),
		context.get_state().config.password_salt.as_bytes(),
		&user.password,
		&argon2::Config {
			variant: Variant::Argon2i,
			hash_length: 64,
			..Default::default()
		},
	)?;

	if !success {
		context.json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::INVALID_PASSWORD,
			request_keys::MESSAGE: error::message::INVALID_PASSWORD
		}));
		return Ok(context);
	}

	// generate JWT
	let iat = get_current_time();
	let exp = iat + (1000 * 3600 * 24 * 3); // 3 days

	let token_data = AccessTokenData::new(iat, exp);
	let jwt = token_data.to_string(context.get_state().config.jwt_secret.as_str())?;
	let refresh_token = Uuid::new_v4();

	db::add_user_login(
		context.get_db_connection(),
		refresh_token.as_bytes().to_vec(),
		iat + (1000 * 60 * 60 * 24 * 30), // 30 days
		user.id,
		iat,
		iat,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ACCESS_TOKEN: jwt,
		request_keys::REFRESH_TOKEN: refresh_token.to_simple().to_string()
	}));
	Ok(context)
}

async fn sign_up(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let body = if let Some(body) = context.get_body_object() {
		body.clone()
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::WRONG_PARAMETERS,
			request_keys::MESSAGE: error::message::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let username = if let Some(Value::String(username)) = body.get(request_keys::USERNAME) {
		username
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::WRONG_PARAMETERS,
			request_keys::MESSAGE: error::message::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let email = if let Some(Value::String(email)) = body.get(request_keys::EMAIL) {
		email
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::WRONG_PARAMETERS,
			request_keys::MESSAGE: error::message::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let password = if let Some(Value::String(password)) = body.get(request_keys::PASSWORD) {
		password
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::WRONG_PARAMETERS,
			request_keys::MESSAGE: error::message::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let account_type =
		if let Some(Value::String(account_type)) = body.get(request_keys::ACCOUNT_TYPE) {
			account_type
		} else {
			context.status(400).json(json!({
				request_keys::SUCCESS: false,
				request_keys::ERROR: error::id::WRONG_PARAMETERS,
				request_keys::MESSAGE: error::message::WRONG_PARAMETERS
			}));
			return Ok(context);
		};

	let first_name = if let Some(Value::String(first_name)) = body.get(request_keys::FIRST_NAME) {
		first_name
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::WRONG_PARAMETERS,
			request_keys::MESSAGE: error::message::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let last_name = if let Some(Value::String(last_name)) = body.get(request_keys::LAST_NAME) {
		last_name
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::WRONG_PARAMETERS,
			request_keys::MESSAGE: error::message::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let (domain_name, organisation_name, backup_email) = match account_type.as_ref() {
		"organisation" => (
			if let Some(Value::String(domain)) = body.get(request_keys::DOMAIN) {
				Some(domain)
			} else {
				context.status(400).json(json!({
					request_keys::SUCCESS: false,
					request_keys::ERROR: error::id::WRONG_PARAMETERS,
					request_keys::MESSAGE: error::message::WRONG_PARAMETERS
				}));
				return Ok(context);
			},
			if let Some(Value::String(organisation_name)) =
				body.get(request_keys::ORGANISATION_NAME)
			{
				Some(organisation_name)
			} else {
				context.status(400).json(json!({
					request_keys::SUCCESS: false,
					request_keys::ERROR: error::id::WRONG_PARAMETERS,
					request_keys::MESSAGE: error::message::WRONG_PARAMETERS
				}));
				return Ok(context);
			},
			if let Some(Value::String(backup_email)) = body.get(request_keys::BACKUP_EMAIL) {
				Some(backup_email)
			} else {
				context.status(400).json(json!({
					request_keys::SUCCESS: false,
					request_keys::ERROR: error::id::WRONG_PARAMETERS,
					request_keys::MESSAGE: error::message::WRONG_PARAMETERS
				}));
				return Ok(context);
			},
		),
		"personal" => (None, None, None),
		_ => {
			context.status(400).json(json!({
				request_keys::SUCCESS: false,
				request_keys::ERROR: error::id::WRONG_PARAMETERS,
				request_keys::MESSAGE: error::message::WRONG_PARAMETERS
			}));
			return Ok(context);
		}
	};

	if !validator::is_username_valid(username) {
		context.json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::INVALID_USERNAME,
			request_keys::MESSAGE: error::message::INVALID_USERNAME
		}));
		return Ok(context);
	}

	if !validator::is_email_valid(email) {
		context.json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::INVALID_EMAIL,
			request_keys::MESSAGE: error::message::INVALID_EMAIL
		}));
		return Ok(context);
	}

	if let Some(backup_email) = backup_email {
		if !validator::is_email_valid(email) {
			context.json(json!({
				request_keys::SUCCESS: false,
				request_keys::ERROR: error::id::INVALID_EMAIL,
				request_keys::MESSAGE: error::message::INVALID_EMAIL
			}));
			return Ok(context);
		}
	}

	if !validator::is_password_valid(password) {
		context.json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::PASSWORD_TOO_WEAK,
			request_keys::MESSAGE: error::message::PASSWORD_TOO_WEAK
		}));
		return Ok(context);
	}

	if db::get_user_by_username(context.get_db_connection(), username)
		.await?
		.is_some()
	{
		context.json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::USERNAME_TAKEN,
			request_keys::MESSAGE: error::message::USERNAME_TAKEN
		}));
		return Ok(context);
	}

	if db::get_user_by_email(context.get_db_connection(), email)
		.await?
		.is_some()
	{
		context.json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::EMAIL_TAKEN,
			request_keys::MESSAGE: error::message::EMAIL_TAKEN
		}));
		return Ok(context);
	}

	let otp: u32 = rand::thread_rng().gen_range(0, 999999);
	let otp = if otp < 10 {
		format!("00000{}", otp)
	} else if otp < 100 {
		format!("0000{}", otp)
	} else if otp < 1000 {
		format!("000{}", otp)
	} else if otp < 10000 {
		format!("00{}", otp)
	} else if otp < 100000 {
		format!("0{}", otp)
	} else {
		format!("{}", otp)
	};
	let otp = format!("{}-{}", &otp[..3], &otp[3..]);

	let token_expiry = get_current_time() + (1000 * 60 * 60 * 2); // 2 hours
	let password = argon2::hash_raw(
		password.as_bytes(),
		context.get_state().config.password_salt.as_bytes(),
		&argon2::Config {
			variant: Variant::Argon2i,
			hash_length: 64,
			..Default::default()
		},
	)?;
	let token_hash = argon2::hash_raw(
		otp.as_bytes(),
		context.get_state().config.password_salt.as_bytes(),
		&argon2::Config {
			variant: Variant::Argon2i,
			hash_length: 64,
			..Default::default()
		},
	)?;

	let email = if account_type == "organisation" {
		UserEmailAddressSignUp::Organisation {
			email_local: email.replace(&format!("@{}", domain_name.unwrap()), ""),
			domain_name: domain_name.unwrap().clone(),
			organisation_name: organisation_name.unwrap().clone(),
			backup_email: backup_email.unwrap().clone(),
		}
	} else if account_type == "personal" {
		UserEmailAddressSignUp::Personal(email.clone())
	} else {
		panic!("email type is neither personal, nor organisation. How did you even get here?")
	};

	db::set_user_to_be_signed_up(
		context.get_db_connection(),
		email.clone(),
		username,
		&password,
		&first_name,
		&last_name,
		&token_hash,
		token_expiry,
	)
	.await?;

	let config = context.get_state().config.clone();
	task::spawn(async move {
		mailer::send_email_verification_mail(
			config,
			match email {
				UserEmailAddressSignUp::Organisation { backup_email, .. } => backup_email,
				UserEmailAddressSignUp::Personal(email) => email,
			},
			otp,
		);
	});

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

async fn join(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let body = if let Some(body) = context.get_body_object() {
		body.clone()
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::WRONG_PARAMETERS,
			request_keys::MESSAGE: error::message::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let otp = if let Some(Value::String(token)) = body.get(request_keys::VERIFICATION_TOKEN) {
		token
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::WRONG_PARAMETERS,
			request_keys::MESSAGE: error::message::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let username = if let Some(Value::String(username)) = body.get(request_keys::USERNAME) {
		username
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::WRONG_PARAMETERS,
			request_keys::MESSAGE: error::message::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let user_data = if let Some(user_data) =
		db::get_user_email_to_sign_up(context.get_db_connection(), username).await?
	{
		user_data
	} else {
		context.json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::INVALID_OTP,
			request_keys::MESSAGE: error::message::INVALID_OTP
		}));
		return Ok(context);
	};

	let success = argon2::verify_raw(
		otp.as_bytes(),
		context.get_state().config.password_salt.as_bytes(),
		&user_data.otp_hash,
		&argon2::Config {
			variant: Variant::Argon2i,
			hash_length: 64,
			..Default::default()
		},
	)?;
	if !success {
		context.json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::INVALID_OTP,
			request_keys::MESSAGE: error::message::INVALID_OTP
		}));
		return Ok(context);
	}

	if user_data.otp_expiry < get_current_time() {
		context.json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::OTP_EXPIRED,
			request_keys::MESSAGE: error::message::OTP_EXPIRED
		}));
		return Ok(context);
	}

	let user_id = Uuid::new_v4();
	let user_id = user_id.as_bytes();

	db::create_user(
		context.get_db_connection(),
		user_id,
		&user_data.username,
		&user_data.password,
		&user_data.backup_email,
		&user_data.first_name,
		&user_data.last_name,
	)
	.await?;

	// For an organisation, create the organisation and domain
	let email;
	let welcome_email_to;
	let backup_email_notification_to;
	match user_data.email {
		UserEmailAddressSignUp::Personal(email_address) => {
			email = UserEmailAddress::Personal(email_address.clone());
			backup_email_notification_to = None;
			welcome_email_to = email_address;
		}
		UserEmailAddressSignUp::Organisation {
			domain_name,
			email_local,
			backup_email,
			organisation_name,
		} => {
			let organisation_id = Uuid::new_v4();
			let organisation_id = organisation_id.as_bytes();

			db::create_organisation(
				context.get_db_connection(),
				organisation_id,
				&organisation_name,
				user_id,
			)
			.await?;

			let domain_id = Uuid::new_v4();
			let domain_id = domain_id.as_bytes().to_vec();

			db::add_domain_to_organisation(
				context.get_db_connection(),
				&domain_id,
				&domain_name,
				organisation_id,
			)
			.await?;

			welcome_email_to = format!("{}@{}", email_local, domain_name);
			email = UserEmailAddress::Organisation {
				domain_id,
				email_local,
			};
			backup_email_notification_to = Some(backup_email);
		}
	}

	db::add_email_for_user(context.get_db_connection(), user_id, email).await?;
	db::delete_user_to_be_signed_up(context.get_db_connection(), &user_data.username).await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));

	let config = context.get_state().config.clone();
	task::spawn(async move {
		mailer::send_sign_up_completed_mail(config, welcome_email_to);
	});

	if let Some(backup_email) = backup_email_notification_to {
		let config = context.get_state().config.clone();
		task::spawn(async move {
			mailer::send_sign_up_completed_mail(config, backup_email);
		});
	}

	Ok(context)
}

async fn get_access_token(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let refresh_token = if let Some(header) = context.get_header("Authorization") {
		header
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::WRONG_PARAMETERS,
			request_keys::MESSAGE: error::message::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let refresh_token = if let Ok(uuid) = Uuid::parse_str(&refresh_token) {
		uuid
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::WRONG_PARAMETERS,
			request_keys::MESSAGE: error::message::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let user_login = db::get_user_login(
		context.get_db_connection(),
		refresh_token.as_bytes().to_vec(),
	)
	.await?;

	if user_login.is_none() {
		context.json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::EMAIL_TOKEN_NOT_FOUND,
			request_keys::MESSAGE: error::message::EMAIL_TOKEN_NOT_FOUND
		}));
		return Ok(context);
	}
	let user_login = user_login.unwrap();

	if user_login.token_expiry < get_current_time() {
		// Token has expired
		context.status(401).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::UNAUTHORIZED,
			request_keys::MESSAGE: error::message::UNAUTHORIZED,
		}));
		return Ok(context);
	}

	// TODO get roles and permissions of user for rbac here
	// use that info to populate the data in the token_data

	let iat = get_current_time();
	let exp = iat + (1000 * 60 * 60 * 24 * 3); // 3 days
	let token_data = AccessTokenData::new(iat, exp);
	let refresh_token = token_data.to_string(&context.get_state().config.jwt_secret)?;

	db::set_refresh_token_expiry(
		context.get_db_connection(),
		refresh_token.as_bytes().to_vec(),
		iat,
		exp,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ACCESS_TOKEN: refresh_token
	}));
	Ok(context)
}

async fn is_email_valid(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let body = if let Some(body) = context.get_body_object() {
		body.clone()
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::WRONG_PARAMETERS,
			request_keys::MESSAGE: error::message::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let email = if let Some(Value::String(email)) = body.get(request_keys::EMAIL) {
		email
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::WRONG_PARAMETERS,
			request_keys::MESSAGE: error::message::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	if !validator::is_email_valid(email) {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::INVALID_EMAIL,
			request_keys::MESSAGE: error::message::INVALID_EMAIL
		}));
		return Ok(context);
	}

	let user = db::get_user_by_email(context.get_db_connection(), email).await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::AVAILABLE: user.is_none()
	}));
	Ok(context)
}

async fn is_username_valid(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let body = if let Some(body) = context.get_body_object() {
		body.clone()
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::WRONG_PARAMETERS,
			request_keys::MESSAGE: error::message::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let username = if let Some(Value::String(username)) = body.get(request_keys::USERNAME) {
		username
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::WRONG_PARAMETERS,
			request_keys::MESSAGE: error::message::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	if !validator::is_username_valid(username) {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::INVALID_USERNAME,
			request_keys::MESSAGE: error::message::INVALID_USERNAME
		}));
		return Ok(context);
	}

	let user = db::get_user_by_username(context.get_db_connection(), username).await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::AVAILABLE: user.is_none()
	}));
	Ok(context)
}
