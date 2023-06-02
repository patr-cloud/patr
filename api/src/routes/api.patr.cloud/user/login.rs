use api_models::{
	models::user::{
		DeleteUserLoginResponse,
		GetUserLoginInfoResponse,
		ListUserLoginsResponse,
		UserWebLogin,
	},
	utils::{DateTime, Location, Uuid},
};
use chrono::{Duration, Utc};
use eve_rs::{App as EveApp, AsError, NextHandler};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	pin_fn,
	redis,
	service::get_access_token_expiry,
	utils::{constants::request_keys, Error, EveContext, EveMiddleware},
};

pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, Error> {
	let mut sub_app = create_eve_app(app);

	sub_app.get(
		"/logins",
		[
			EveMiddleware::PlainTokenAuthenticator {
				is_api_token_allowed: false,
			},
			EveMiddleware::CustomFunction(pin_fn!(get_all_logins_for_user)),
		],
	);
	sub_app.get(
		"/logins/:loginId/info",
		[
			EveMiddleware::PlainTokenAuthenticator {
				is_api_token_allowed: false,
			},
			EveMiddleware::CustomFunction(pin_fn!(get_login_info)),
		],
	);
	sub_app.delete(
		"/logins/:loginId",
		[
			EveMiddleware::PlainTokenAuthenticator {
				is_api_token_allowed: false,
			},
			EveMiddleware::CustomFunction(pin_fn!(delete_user_login)),
		],
	);

	sub_app
}

async fn get_all_logins_for_user(
	mut context: EveContext,
	_: NextHandler<EveContext, Error>,
) -> Result<EveContext, Error> {
	let user_id = context.get_token_data().unwrap().user_id().clone();

	let logins = db::get_all_web_logins_for_user(
		context.get_database_connection(),
		&user_id,
	)
	.await?
	.into_iter()
	.map(|login| UserWebLogin {
		login_id: login.login_id,
		token_expiry: DateTime(login.token_expiry),
		created: DateTime(login.created),
		created_ip: login.created_ip,
		created_location: Location {
			lat: login.created_location_latitude,
			lng: login.created_location_longitude,
		},
		created_country: login.created_country,
		created_region: login.created_region,
		created_city: login.created_city,
		created_timezone: login.created_timezone,
		last_login: DateTime(login.last_login),
		last_activity: DateTime(login.last_activity),
		last_activity_ip: login.last_activity_ip,
		last_activity_location: Location {
			lat: login.last_activity_location_latitude,
			lng: login.last_activity_location_longitude,
		},
		last_activity_user_agent: login.last_activity_user_agent,
		last_activity_country: login.last_activity_country,
		last_activity_region: login.last_activity_region,
		last_activity_city: login.last_activity_city,
		last_activity_timezone: login.last_activity_timezone,
	})
	.collect::<Vec<_>>();

	context.success(ListUserLoginsResponse { logins }).await?;
	Ok(context)
}

async fn get_login_info(
	mut context: EveContext,
	_: NextHandler<EveContext, Error>,
) -> Result<EveContext, Error> {
	let login_id = context
		.get_param(request_keys::LOGIN_ID)
		.and_then(|param| Uuid::parse_str(param).ok())
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let login =
		db::get_user_web_login(context.get_database_connection(), &login_id)
			.await?
			.map(|login| UserWebLogin {
				login_id: login.login_id,
				token_expiry: DateTime(login.token_expiry),
				created: DateTime(login.created),
				created_ip: login.created_ip,
				created_location: Location {
					lat: login.created_location_latitude,
					lng: login.created_location_longitude,
				},
				created_country: login.created_country,
				created_region: login.created_region,
				created_city: login.created_city,
				created_timezone: login.created_timezone,
				last_login: DateTime(login.last_login),
				last_activity: DateTime(login.last_activity),
				last_activity_ip: login.last_activity_ip,
				last_activity_location: Location {
					lat: login.last_activity_location_latitude,
					lng: login.last_activity_location_longitude,
				},
				last_activity_user_agent: login.last_activity_user_agent,
				last_activity_country: login.last_activity_country,
				last_activity_region: login.last_activity_region,
				last_activity_city: login.last_activity_city,
				last_activity_timezone: login.last_activity_timezone,
			})
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?;

	context.success(GetUserLoginInfoResponse { login }).await?;
	Ok(context)
}

async fn delete_user_login(
	mut context: EveContext,
	_: NextHandler<EveContext, Error>,
) -> Result<EveContext, Error> {
	let login_id = context
		.get_param(request_keys::LOGIN_ID)
		.and_then(|param| Uuid::parse_str(param).ok())
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let user_id = context.get_token_data().unwrap().user_id().clone();

	db::delete_user_web_login_by_id(
		context.get_database_connection(),
		&login_id,
		&user_id,
	)
	.await?;

	let ttl = get_access_token_expiry() + Duration::hours(2); // 2 hrs buffer time
	redis::revoke_login_tokens_created_before_timestamp(
		context.get_redis_connection(),
		&login_id,
		&Utc::now(),
		Some(&ttl),
	)
	.await?;

	context.success(DeleteUserLoginResponse {}).await?;
	Ok(context)
}
