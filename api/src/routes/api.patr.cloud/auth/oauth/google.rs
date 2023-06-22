use std::net::{IpAddr, Ipv4Addr};

use ::redis::AsyncCommands;
use api_models::{
	models::auth::{
		GoogleAuthorizeResponse,
		GoogleOAuthCallbackRequest,
		GoogleOAuthCallbackResponse,
		RecoveryMethod,
		SignUpAccountType,
	},
	utils::{Personal, True},
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use reqwest::header;
use url::Url;

use crate::{
	app::{create_eve_app, App},
	db::{self, UserWebLogin},
	error,
	models::google::GoogleUserInfo,
	pin_fn,
	routes,
	service,
	utils::{
		constants::google_oauth,
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
		"/authorize",
		[EveMiddleware::CustomFunction(pin_fn!(
			authorize_with_google
		))],
	);
	app.post(
		"/callback",
		[EveMiddleware::CustomFunction(pin_fn!(oauth_callback))],
	);

	app
}

async fn authorize_with_google(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let state = thread_rng()
		.sample_iter(&Alphanumeric)
		.take(8)
		.map(char::from)
		.collect::<String>();

	context
		.get_redis_connection()
		.set_ex(
			format!("googleOAuthState:{}", state),
			"true".to_owned(),
			60 * 5,
		) // 5 minutes
		.await?;

	context.success(GoogleAuthorizeResponse {
		oauth_url: Url::parse_with_params(
			google_oauth::AUTH_URL,
			&[
				(
					"client_id",
					context.get_state().config.google.client_id.as_str(),
				),
				("scope", google_oauth::SCOPE),
				("state", state.as_str()),
				("response_type", "token"),
				("redirect_uri", google_oauth::REDIRECT_URL),
				("include_granted_scopes", "true"),
			],
		)?
		.to_string(),
	});

	Ok(context)
}

async fn oauth_callback(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let GoogleOAuthCallbackRequest {
		token,
		state,
		username,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	// Check if the state is correct and request is not forged
	let redis_google_state: Option<String> = context
		.get_redis_connection()
		.get(format!("googleOAuthState:{}", state))
		.await?;

	if redis_google_state.is_none() {
		Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?
	}

	let client = reqwest::Client::new();
	log::trace!("Getting user information");

	let GoogleUserInfo { name, email } = client
		.get(google_oauth::USER_INFO_URL)
		.header(header::AUTHORIZATION, format!("Bearer {}", token))
		.header(header::ACCEPT, "application/json")
		.send()
		.await?
		.error_for_status()?
		.json::<GoogleUserInfo>()
		.await?;

	let existing_user =
		db::get_user_by_email(context.get_database_connection(), &email)
			.await?;

	let response = if let Some(user) = existing_user {
		let ip_address = routes::get_request_ip_address(&context);
		let user_agent = context.get_header("user-agent").unwrap_or_default();
		let config = context.get_state().config.clone();
		log::trace!("Get access token for user sign in");
		let (UserWebLogin { login_id, .. }, access_token, refresh_token) =
			service::sign_in_user(
				context.get_database_connection(),
				&user.id,
				&ip_address
					.parse()
					.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
				&user_agent,
				&config,
			)
			.await?;
		log::trace!("Login success");

		GoogleOAuthCallbackResponse::Login {
			login_id,
			access_token,
			refresh_token,
		}
	} else {
		let username = username
			.status(404)
			.body(error!(INVALID_USERNAME).to_string())?;

		let user_exist = db::get_user_by_username(
			context.get_database_connection(),
			&username,
		)
		.await?;

		if user_exist.is_some() {
			return Err(Error::empty()
				.status(404)
				.body(error!(USERNAME_TAKEN).to_string()));
		}

		let (first_name, last_name) =
			name.split_once(' ').unwrap_or((name.as_str(), ""));
		log::trace!("Creating join request for user");
		let (user_to_sign_up, otp) = service::create_user_join_request(
			context.get_database_connection(),
			username.to_lowercase().trim(),
			"",
			first_name,
			last_name,
			&SignUpAccountType::Personal {
				account_type: Personal,
			},
			&RecoveryMethod::Email {
				recovery_email: email,
			},
			None,
			true,
		)
		.await?;

		log::trace!("Sending otp to user's primary mail");
		service::send_user_sign_up_otp(
			context.get_database_connection(),
			&user_to_sign_up,
			&otp,
		)
		.await?;

		let _ = service::get_internal_metrics(
			context.get_database_connection(),
			"A new user has attempted to sign-up",
		)
		.await;
		log::trace!("Registration success");
		GoogleOAuthCallbackResponse::SignUp {
			verification_required: True,
		}
	};

	context.success(response);
	Ok(context)
}
