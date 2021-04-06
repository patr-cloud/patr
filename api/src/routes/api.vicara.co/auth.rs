use crate::{
	app::{create_eve_app, App},
	db, error,
	models::{
		db_mapping::{UserEmailAddress, UserEmailAddressSignUp},
		rbac, AccessTokenData, ExposedUserData,
	},
	pin_fn, service,
	utils::{
		self, constants::request_keys, get_current_time, mailer, validator,
		EveContext, EveMiddleware,
	},
};

use argon2::Variant;
use eve_rs::{App as EveApp, Context, Error, NextHandler};
use serde_json::{json, Value};
use tokio::task;
use uuid::Uuid;

pub fn create_sub_app(app: &App) -> EveApp<EveContext, EveMiddleware, App> {
	let mut app = create_eve_app(&app);

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
	app.post(
		"/forgot-password",
		&[EveMiddleware::CustomFunction(pin_fn!(forgot_password))],
	);
	app.post(
		"/reset-password",
		&[EveMiddleware::CustomFunction(pin_fn!(reset_password))],
	);

	app
}

async fn sign_in(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let body = context.get_body_object().clone();

	let user_id =
		if let Some(Value::String(user_id)) = body.get(request_keys::USER_ID) {
			user_id
		} else {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		};

	let password = if let Some(Value::String(password)) =
		body.get(request_keys::PASSWORD)
	{
		password
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let user = if let Some(user) = db::get_user_by_username_or_email(
		context.get_mysql_connection(),
		&user_id,
	)
	.await?
	{
		user
	} else {
		context.json(error!(USER_NOT_FOUND));
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
		context.json(error!(INVALID_PASSWORD));
		return Ok(context);
	}

	// generate JWT
	let iat = get_current_time();
	let exp = iat + (1000 * 3600 * 24 * 3); // 3 days
	let orgs = db::get_all_organisation_roles_for_user(
		context.get_mysql_connection(),
		&user.id,
	)
	.await?;
	let user = ExposedUserData {
		id: user.id,
		username: user.username,
		first_name: user.first_name,
		last_name: user.last_name,
		created: user.created,
	};

	let token_data = AccessTokenData::new(iat, exp, orgs, user);
	let jwt =
		token_data.to_string(context.get_state().config.jwt_secret.as_str())?;
	let refresh_token = Uuid::new_v4();

	db::add_user_login(
		context.get_mysql_connection(),
		refresh_token.as_bytes(),
		iat + (1000 * 60 * 60 * 24 * 30), // 30 days
		&token_data.user.id,
		iat,
		iat,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ACCESS_TOKEN: jwt,
		request_keys::REFRESH_TOKEN: refresh_token.to_simple().to_string().to_lowercase()
	}));
	Ok(context)
}

async fn sign_up(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let body = context.get_body_object().clone();

	let username = if let Some(Value::String(username)) =
		body.get(request_keys::USERNAME)
	{
		username
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let email =
		if let Some(Value::String(email)) = body.get(request_keys::EMAIL) {
			email
		} else {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		};

	let password = if let Some(Value::String(password)) =
		body.get(request_keys::PASSWORD)
	{
		password
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let account_type = if let Some(Value::String(account_type)) =
		body.get(request_keys::ACCOUNT_TYPE)
	{
		account_type
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let first_name = if let Some(Value::String(first_name)) =
		body.get(request_keys::FIRST_NAME)
	{
		first_name
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let last_name = if let Some(Value::String(last_name)) =
		body.get(request_keys::LAST_NAME)
	{
		last_name
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let (domain_name, organisation_name, backup_email) = match account_type
		.as_ref()
	{
		"organisation" => (
			if let Some(Value::String(domain)) = body.get(request_keys::DOMAIN)
			{
				Some(domain)
			} else {
				context.status(400).json(error!(WRONG_PARAMETERS));
				return Ok(context);
			},
			if let Some(Value::String(organisation_name)) =
				body.get(request_keys::ORGANISATION_NAME)
			{
				Some(organisation_name)
			} else {
				context.status(400).json(error!(WRONG_PARAMETERS));
				return Ok(context);
			},
			if let Some(Value::String(backup_email)) =
				body.get(request_keys::BACKUP_EMAIL)
			{
				Some(backup_email)
			} else {
				context.status(400).json(error!(WRONG_PARAMETERS));
				return Ok(context);
			},
		),
		"personal" => (None, None, None),
		_ => {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		}
	};

	let config = context.get_state().config.clone();
	let user_to_be_signed_up = service::create_user_to_be_signed_up(
		context.get_mysql_connection(),
		&config,
		&username,
		&email,
		&password,
		&account_type,
		domain_name,
		organisation_name,
		backup_email,
		&first_name,
		&last_name,
	)
	.await;

	if let Err(err) = user_to_be_signed_up {
		context.json(err);
		return Ok(context);
	}
	let otp = user_to_be_signed_up.unwrap();

	let email = email.clone();
	task::spawn_blocking(|| {
		mailer::send_email_verification_mail(config, email, otp);
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
	let body = context.get_body_object().clone();

	let otp = if let Some(Value::String(token)) =
		body.get(request_keys::VERIFICATION_TOKEN)
	{
		token
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let username = if let Some(Value::String(username)) =
		body.get(request_keys::USERNAME)
	{
		username
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let user_data = if let Some(user_data) =
		db::get_user_email_to_sign_up(context.get_mysql_connection(), &username)
			.await?
	{
		user_data
	} else {
		context.json(error!(INVALID_OTP));
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
		context.json(error!(INVALID_OTP));
		return Ok(context);
	}

	if user_data.otp_expiry < get_current_time() {
		context.json(error!(OTP_EXPIRED));
		return Ok(context);
	}

	// For a personal account, get:
	// - username
	// - email
	// - password
	// - account_type
	// - first_name
	// - last_name

	// For an organisation account, also get:
	// - domain_name
	// - organisation_name
	// - backup_email

	// First create user,
	// Then create an organisation if an org account,
	// Then add the domain if org account,
	// Then create personal org regardless,
	// Then set email to backup email if personal account,
	// And finally send the token, along with the email to the user

	let user_uuid = Uuid::new_v4();
	let user_id = user_uuid.as_bytes();
	let created = get_current_time();

	if rbac::GOD_USER_ID.get().is_none() {
		rbac::GOD_USER_ID
			.set(user_uuid)
			.expect("GOD_USER_ID was already set");
	}

	db::create_user(
		context.get_mysql_connection(),
		user_id,
		&user_data.username,
		&user_data.password,
		&user_data.backup_email,
		(&user_data.first_name, &user_data.last_name),
		created,
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
			let organisation_id =
				db::generate_new_resource_id(context.get_mysql_connection())
					.await?;
			let organisation_id = organisation_id.as_bytes();
			db::create_orphaned_resource(
				context.get_mysql_connection(),
				organisation_id,
				&format!("Organiation: {}", organisation_name),
				rbac::RESOURCE_TYPES
					.get()
					.unwrap()
					.get(rbac::resource_types::ORGANISATION)
					.unwrap(),
			)
			.await?;
			db::create_organisation(
				context.get_mysql_connection(),
				organisation_id,
				&organisation_name,
				user_id,
				get_current_time(),
			)
			.await?;
			db::set_resource_owner_id(
				context.get_mysql_connection(),
				organisation_id,
				organisation_id,
			)
			.await?;

			let domain_id =
				db::generate_new_resource_id(context.get_mysql_connection())
					.await?;
			let domain_id = domain_id.as_bytes().to_vec();

			db::create_resource(
				context.get_mysql_connection(),
				&domain_id,
				&format!("Domain: {}", domain_name),
				rbac::RESOURCE_TYPES
					.get()
					.unwrap()
					.get(rbac::resource_types::DOMAIN)
					.unwrap(),
				organisation_id,
			)
			.await?;
			db::add_domain_to_organisation(
				context.get_mysql_connection(),
				&domain_id,
				&domain_name,
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

	// add personal organisation
	let organisation_id =
		db::generate_new_resource_id(context.get_mysql_connection()).await?;
	let organisation_id = organisation_id.as_bytes();
	let organisation_name =
		format!("personal-organisation-{}", hex::encode(user_id));

	db::create_orphaned_resource(
		context.get_mysql_connection(),
		organisation_id,
		&organisation_name,
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::ORGANISATION)
			.unwrap(),
	)
	.await?;
	db::create_organisation(
		context.get_mysql_connection(),
		organisation_id,
		&organisation_name,
		user_id,
		get_current_time(),
	)
	.await?;
	db::set_resource_owner_id(
		context.get_mysql_connection(),
		organisation_id,
		organisation_id,
	)
	.await?;

	db::add_email_for_user(context.get_mysql_connection(), user_id, email)
		.await?;
	db::delete_user_to_be_signed_up(
		context.get_mysql_connection(),
		&user_data.username,
	)
	.await?;

	// generate JWT
	let iat = get_current_time();
	let exp = iat + (1000 * 3600 * 24 * 3); // 3 days
	let orgs = db::get_all_organisation_roles_for_user(
		context.get_mysql_connection(),
		user_id,
	)
	.await?;
	let user = ExposedUserData {
		id: user_id.to_vec(),
		username: user_data.username,
		first_name: user_data.first_name,
		last_name: user_data.last_name,
		created,
	};

	let token_data = AccessTokenData::new(iat, exp, orgs, user);
	let jwt =
		token_data.to_string(context.get_state().config.jwt_secret.as_str())?;
	let refresh_token = Uuid::new_v4();

	db::add_user_login(
		context.get_mysql_connection(),
		refresh_token.as_bytes(),
		iat + (1000 * 60 * 60 * 24 * 30), // 30 days
		user_id,
		iat,
		iat,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ACCESS_TOKEN: jwt,
		request_keys::REFRESH_TOKEN: refresh_token.to_simple().to_string().to_lowercase()
	}));

	let config = context.get_state().config.clone();
	task::spawn_blocking(|| {
		mailer::send_sign_up_completed_mail(config, welcome_email_to);
	});

	if let Some(backup_email) = backup_email_notification_to {
		let config = context.get_state().config.clone();
		task::spawn_blocking(|| {
			mailer::send_backup_registration_mail(config, backup_email);
		});
	}

	Ok(context)
}

async fn get_access_token(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let refresh_token =
		if let Some(header) = context.get_header("Authorization") {
			header
		} else {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		};

	let refresh_token = if let Ok(uuid) = Uuid::parse_str(&refresh_token) {
		uuid
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};
	let refresh_token = refresh_token.as_bytes();

	let user_login =
		db::get_user_login(context.get_mysql_connection(), refresh_token)
			.await?;

	if user_login.is_none() {
		context.json(error!(EMAIL_TOKEN_NOT_FOUND));
		return Ok(context);
	}
	let user_login = user_login.unwrap();

	if user_login.token_expiry < get_current_time() {
		// Token has expired
		context.status(401).json(error!(UNAUTHORIZED));
		return Ok(context);
	}

	// get roles and permissions of user for rbac here
	// use that info to populate the data in the token_data

	let iat = get_current_time();
	let exp = iat + (1000 * 60 * 60 * 24 * 3); // 3 days
	let orgs = db::get_all_organisation_roles_for_user(
		context.get_mysql_connection(),
		&user_login.user_id,
	)
	.await?;
	let user_id = user_login.user_id;
	let user_data =
		db::get_user_by_user_id(context.get_mysql_connection(), &user_id)
			.await?
			.unwrap();
	let user = ExposedUserData {
		id: user_id,
		username: user_data.username,
		first_name: user_data.first_name,
		last_name: user_data.last_name,
		created: user_data.created,
	};
	let token_data = AccessTokenData::new(iat, exp, orgs, user);

	let access_token =
		token_data.to_string(&context.get_state().config.jwt_secret)?;

	db::set_refresh_token_expiry(
		context.get_mysql_connection(),
		refresh_token,
		iat,
		exp,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ACCESS_TOKEN: access_token
	}));
	Ok(context)
}

async fn is_email_valid(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let query = context.get_request().get_query().clone();

	let email = if let Some(email) = query.get(request_keys::EMAIL) {
		email
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	if !validator::is_email_valid(&email) {
		context.status(400).json(error!(INVALID_EMAIL));
		return Ok(context);
	}

	let user =
		db::get_user_by_email(context.get_mysql_connection(), &email).await?;

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
	let query = context.get_request().get_query().clone();

	let username = if let Some(username) = query.get(request_keys::USERNAME) {
		username
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	if !validator::is_username_valid(&username) {
		context.status(400).json(error!(INVALID_USERNAME));
		return Ok(context);
	}

	let user =
		db::get_user_by_username(context.get_mysql_connection(), &username)
			.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::AVAILABLE: user.is_none()
	}));
	Ok(context)
}

async fn forgot_password(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let body = context.get_body_object().clone();

	let user_id =
		if let Some(Value::String(user_id)) = body.get(request_keys::USER_ID) {
			user_id
		} else {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		};

	let user = db::get_user_by_username_or_email(
		context.get_mysql_connection(),
		&user_id,
	)
	.await?;

	if user.is_none() {
		context.json(error!(USER_NOT_FOUND));
		return Ok(context);
	}
	let user = user.unwrap();

	let otp = utils::generate_new_otp();
	let otp = format!("{}-{}", &otp[..3], &otp[3..]);

	let token_expiry = get_current_time() + (1000 * 60 * 60 * 2); // 2 hours

	let token_hash = service::get_hash(
		otp.as_bytes(),
		context.get_state().config.password_salt.as_bytes(),
	)?;

	db::add_password_reset_request(
		context.get_mysql_connection(),
		&user.id,
		&token_hash,
		token_expiry,
	)
	.await?;

	let config = context.get_state().config.clone();
	task::spawn_blocking(|| {
		mailer::send_password_reset_requested_mail(
			config,
			user.backup_email,
			otp,
		);
	});

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

async fn reset_password(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let body = context.get_body_object().clone();

	let new_password = if let Some(Value::String(password)) =
		body.get(request_keys::PASSWORD)
	{
		password
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};
	let token = if let Some(Value::String(token)) =
		body.get(request_keys::VERIFICATION_TOKEN)
	{
		token
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};
	let user_id =
		if let Some(Value::String(user_id)) = body.get(request_keys::USER_ID) {
			user_id
		} else {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		};
	let user_id = if let Ok(user_id) = hex::decode(user_id) {
		user_id
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let reset_request = db::get_password_reset_request_for_user(
		context.get_mysql_connection(),
		&user_id,
	)
	.await?;

	if reset_request.is_none() {
		context.status(400).json(error!(EMAIL_TOKEN_NOT_FOUND));
		return Ok(context);
	}
	let reset_request = reset_request.unwrap();

	let success = argon2::verify_raw(
		token.as_bytes(),
		context.get_state().config.password_salt.as_bytes(),
		&reset_request.token,
		&argon2::Config {
			variant: Variant::Argon2i,
			hash_length: 64,
			..Default::default()
		},
	)?;

	if !success {
		context.status(400).json(error!(EMAIL_TOKEN_NOT_FOUND));
		return Ok(context);
	}

	let new_password = argon2::hash_raw(
		new_password.as_bytes(),
		context.get_state().config.password_salt.as_bytes(),
		&argon2::Config {
			variant: Variant::Argon2i,
			hash_length: 64,
			..Default::default()
		},
	)?;

	db::update_user_password(
		context.get_mysql_connection(),
		&user_id,
		&new_password,
	)
	.await?;
	db::delete_password_reset_request_for_user(
		context.get_mysql_connection(),
		&user_id,
	)
	.await?;

	let config = context.get_state().config.clone();
	let pool = context.get_state().mysql.clone();
	task::spawn(async move {
		let mut connection = pool
			.begin()
			.await
			.expect("unable to begin transaction from connection");
		let user = db::get_user_by_user_id(&mut connection, &user_id)
			.await
			.expect("unable to get user data")
			.expect("user data for that user_id was None");

		task::spawn_blocking(|| {
			mailer::send_password_changed_notification_mail(
				config,
				user.backup_email,
			);
		});
	});

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}
