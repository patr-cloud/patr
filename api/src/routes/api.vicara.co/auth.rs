use crate::{
	app::{create_eve_app, App},
	db, error,
	models::{
		db_mapping::{UserEmailAddress, UserEmailAddressSignUp},
		rbac, AccessTokenData, ExposedUserData, RegistryToken,
		RegistryTokenAccess,
	},
	pin_fn,
	utils::{
		self, constants::request_keys, get_current_time_millis, mailer,
		validator, EveContext, EveMiddleware,
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
	app.get(
		"/docker-registry-token",
		&[EveMiddleware::CustomFunction(pin_fn!(
			docker_registry_token_endpoint
		))],
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
	let iat = get_current_time_millis();
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

	if !validator::is_username_valid(&username) {
		context.json(error!(INVALID_USERNAME));
		return Ok(context);
	}

	if !validator::is_email_valid(&email) {
		context.json(error!(INVALID_EMAIL));
		return Ok(context);
	}

	if backup_email.is_some()
		&& !validator::is_email_valid(backup_email.as_ref().unwrap())
	{
		context.json(error!(INVALID_EMAIL));
		return Ok(context);
	}

	if !validator::is_password_valid(&password) {
		context.json(error!(PASSWORD_TOO_WEAK));
		return Ok(context);
	}

	if let Some(domain) = domain_name {
		if !validator::is_domain_name_valid(domain.as_str()).await {
			context.json(error!(INVALID_DOMAIN_NAME));
			return Ok(context);
		}
	}

	if db::get_user_by_username(context.get_mysql_connection(), &username)
		.await?
		.is_some()
	{
		context.json(error!(USERNAME_TAKEN));
		return Ok(context);
	}

	if db::get_user_by_email(context.get_mysql_connection(), &email)
		.await?
		.is_some()
	{
		context.json(error!(EMAIL_TAKEN));
		return Ok(context);
	}

	let otp = utils::generate_new_otp();
	let otp = format!("{}-{}", &otp[..3], &otp[3..]);

	let token_expiry = get_current_time_millis() + (1000 * 60 * 60 * 2); // 2 hours
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
			email_local: email
				.replace(&format!("@{}", domain_name.unwrap()), ""),
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
		context.get_mysql_connection(),
		email.clone(),
		&username,
		&password,
		(&first_name, &last_name),
		&token_hash,
		token_expiry,
	)
	.await?;

	let config = context.get_state().config.clone();
	task::spawn_blocking(|| {
		mailer::send_email_verification_mail(
			config,
			match email {
				UserEmailAddressSignUp::Organisation {
					backup_email, ..
				} => backup_email,
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

	if user_data.otp_expiry < get_current_time_millis() {
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
	let created = get_current_time_millis();

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
				get_current_time_millis(),
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
		get_current_time_millis(),
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
	let iat = get_current_time_millis();
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

	if user_login.token_expiry < get_current_time_millis() {
		// Token has expired
		context.status(401).json(error!(UNAUTHORIZED));
		return Ok(context);
	}

	// get roles and permissions of user for rbac here
	// use that info to populate the data in the token_data

	let iat = get_current_time_millis();
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

	let token_expiry = get_current_time_millis() + (1000 * 60 * 60 * 2); // 2 hours
	let token_hash = argon2::hash_raw(
		otp.as_bytes(),
		context.get_state().config.password_salt.as_bytes(),
		&argon2::Config {
			variant: Variant::Argon2i,
			hash_length: 64,
			..Default::default()
		},
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

async fn docker_registry_token_endpoint(
	context: EveContext,
	next: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let query = context.get_request().get_query();

	if query.get(request_keys::SCOPE).is_some() {
		// Authenticating an existing login
		docker_registry_authenticate(context, next).await
	} else {
		// Logging in
		docker_registry_login(context, next).await
	}
}

async fn docker_registry_login(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let query = context.get_request().get_query().clone();
	let config = context.get_state().config.clone();

	let _client_id = if let Some(client_id) =
		query.get(request_keys::SNAKE_CASE_CLIENT_ID)
	{
		client_id
	} else {
		context.status(400).json(json!({
			request_keys::ERRORS: [{
				request_keys::CODE: "UNAUTHORIZED",
				request_keys::MESSAGE: "Invalid request sent by the client. Could not find client_id.",
				request_keys::DETAIL: []
			}]
		}));
		return Ok(context);
	};

	let offline_token = if let Some(offline_token) =
		query.get(request_keys::SNAKE_CASE_OFFLINE_TOKEN)
	{
		offline_token
	} else {
		context.status(400).json(json!({
			request_keys::ERRORS: [{
				request_keys::CODE: "UNAUTHORIZED",
				request_keys::MESSAGE: "Invalid request sent by the client. Could not find offline_token.",
				request_keys::DETAIL: []
			}]
		}));
		return Ok(context);
	};

	let _offline_token = if let Ok(value) = offline_token.parse::<bool>() {
		value
	} else {
		context.status(400).json(json!({
			request_keys::ERRORS: [{
				request_keys::CODE: "UNAUTHORIZED",
				request_keys::MESSAGE: "Invalid request sent by the client. offline_token is not a boolean",
				request_keys::DETAIL: []
			}]
		}));
		return Ok(context);
	};

	let service = if let Some(service) = query.get(request_keys::SERVICE) {
		service
	} else {
		context.status(400).json(json!({
			request_keys::ERRORS: [{
				request_keys::CODE: "UNAUTHORIZED",
				request_keys::MESSAGE: "Invalid request sent by the client. Could not find service.",
				request_keys::DETAIL: []
			}]
		}));
		return Ok(context);
	};

	if service != &config.docker_registry.service_name {
		context.status(400).json(json!({
			request_keys::ERRORS: [{
				request_keys::CODE: "UNAUTHORIZED",
				request_keys::MESSAGE: "Invalid request sent by the client. Service is not valid.",
				request_keys::DETAIL: []
			}]
		}));
		return Ok(context);
	}

	if context.get_header("Authorization").is_none() {
		context.status(400).json(json!({
			request_keys::ERRORS: [{
				request_keys::CODE: "UNAUTHORIZED",
				request_keys::MESSAGE: "Invalid request sent by the client. Authorization header not found.",
				request_keys::DETAIL: []
			}]
		}));
		return Ok(context);
	}
	let authorization = context.get_header("Authorization").unwrap();
	let authorization = authorization.replace("Basic ", "");
	let authorization = if let Ok(data) = base64::decode(authorization) {
		if let Ok(data) = String::from_utf8(data) {
			data
		} else {
			context.status(400).json(json!({
				request_keys::ERRORS: [{
					request_keys::CODE: "UNAUTHORIZED",
					request_keys::MESSAGE: "Invalid request sent by the client. Authorization data could not be converted to a string.",
					request_keys::DETAIL: []
				}]
			}));
			return Ok(context);
		}
	} else {
		context.status(400).json(json!({
			request_keys::ERRORS: [{
				request_keys::CODE: "UNAUTHORIZED",
				request_keys::MESSAGE: "Invalid request sent by the client. Authorization header could not be base64 decoded.",
				request_keys::DETAIL: []
			}]
		}));
		return Ok(context);
	};

	let mut splitter = authorization.split(':');
	let (username, password) = {
		let username = if let Some(username) = splitter.next() {
			username
		} else {
			context.status(400).json(json!({
				request_keys::ERRORS: [{
					request_keys::CODE: "UNAUTHORIZED",
					request_keys::MESSAGE: "Invalid request sent by the client. Authorization header did not have username.",
					request_keys::DETAIL: []
				}]
			}));
			return Ok(context);
		};

		let password = if let Some(password) = splitter.next() {
			password
		} else {
			context.status(400).json(json!({
				request_keys::ERRORS: [{
					request_keys::CODE: "UNAUTHORIZED",
					request_keys::MESSAGE: "Invalid request sent by the client. Authorization header did not have password.",
					request_keys::DETAIL: []
				}]
			}));
			return Ok(context);
		};
		(username, password)
	};

	let user = if let Some(user) =
		db::get_user_by_username(context.get_mysql_connection(), &username)
			.await?
	{
		user
	} else {
		context.status(401).json(json!({
			request_keys::ERRORS: [{
				request_keys::CODE: "UNAUTHORIZED",
				request_keys::MESSAGE: "User not found.",
				request_keys::DETAIL: []
			}]
		}));
		return Ok(context);
	};

	let success = argon2::verify_raw(
		password.as_bytes(),
		config.password_salt.as_bytes(),
		&user.password,
		&argon2::Config {
			variant: Variant::Argon2i,
			hash_length: 64,
			..Default::default()
		},
	)?;

	if !success {
		context.status(401).json(json!({
			request_keys::ERRORS: [{
				request_keys::CODE: "UNAUTHORIZED",
				request_keys::MESSAGE: "Password invalid",
				request_keys::DETAIL: []
			}]
		}));
		return Ok(context);
	}

	let token = RegistryToken::new(
		if cfg!(debug_assertions) {
			format!("localhost:{}", config.port)
		} else {
			"api.vicara.co".to_string()
		},
		username.to_string(),
		&config,
		vec![],
	)
	.to_string(
		config.docker_registry.private_key.as_ref(),
		config.docker_registry.public_key_der(),
	)?;

	context.json(json!({
		request_keys::TOKEN: token,
		request_keys::REFRESH_TOKEN: "test",
	}));
	return Ok(context);
}

async fn docker_registry_authenticate(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let query = context.get_request().get_query().clone();
	let config = context.get_state().config.clone();

	let authorization = if let Some(token) = context.get_header("Authorization")
	{
		token
	} else {
		context.status(401).json(json!({
				request_keys::ERRORS: [{
					request_keys::CODE: "UNAUTHORIZED",
					request_keys::MESSAGE: format!("Please login to {} first", config.docker_registry.issuer),
					request_keys::DETAIL: []
				}]
			}));
		return Ok(context);
	};

	let token = authorization.replace("Basic ", "");

	// TODO fix this crap
	let auth = String::from_utf8(base64::decode(token)?)?;

	let mut splitter = auth.split(':');
	let (username, password) = {
		let username = if let Some(username) = splitter.next() {
			username
		} else {
			context.status(400).json(json!({
					request_keys::ERRORS: [{
						request_keys::CODE: "UNAUTHORIZED",
						request_keys::MESSAGE: "Invalid request sent by the client. Authorization header did not have username.",
						request_keys::DETAIL: []
					}]
				}));
			return Ok(context);
		};

		let password = if let Some(password) = splitter.next() {
			password
		} else {
			context.status(400).json(json!({
					request_keys::ERRORS: [{
						request_keys::CODE: "UNAUTHORIZED",
						request_keys::MESSAGE: "Invalid request sent by the client. Authorization header did not have password.",
						request_keys::DETAIL: []
					}]
				}));
			return Ok(context);
		};
		(username, password)
	};

	let user = if let Some(user) =
		db::get_user_by_username(context.get_mysql_connection(), &username)
			.await?
	{
		user
	} else {
		context.status(401).json(json!({
			request_keys::ERRORS: [{
				request_keys::CODE: "UNAUTHORIZED",
				request_keys::MESSAGE: "User not found.",
				request_keys::DETAIL: []
			}]
		}));
		return Ok(context);
	};

	let success = argon2::verify_raw(
		password.as_bytes(),
		config.password_salt.as_bytes(),
		&user.password,
		&argon2::Config {
			variant: Variant::Argon2i,
			hash_length: 64,
			..Default::default()
		},
	)?;

	if !success {
		context.status(401).json(json!({
			request_keys::ERRORS: [{
				request_keys::CODE: "UNAUTHORIZED",
				request_keys::MESSAGE: "Password invalid",
				request_keys::DETAIL: []
			}]
		}));
		return Ok(context);
	}

	let scope = query.get(request_keys::SCOPE).unwrap();
	let mut splitter = scope.split(':');
	let access_type = splitter.next().unwrap();
	let repo = splitter.next().unwrap();
	let action = splitter.next().unwrap();

	let token = RegistryToken::new(
		if cfg!(debug_assertions) {
			format!("localhost:{}", config.port)
		} else {
			"api.vicara.co".to_string()
		},
		username.to_string(),
		&config,
		vec![RegistryTokenAccess {
			r#type: access_type.to_string(),
			name: repo.to_string(),
			actions: vec![action.to_string()],
		}],
	)
	.to_string(
		config.docker_registry.private_key.as_ref(),
		config.docker_registry.public_key_der(),
	)?;

	context.json(json!({
		request_keys::TOKEN: token,
		request_keys::REFRESH_TOKEN: "test",
	}));
	return Ok(context);
}
