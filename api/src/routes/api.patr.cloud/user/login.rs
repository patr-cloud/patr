use api_models::{
	models::user::{
		DeleteUserLoginRequest,
		GetUserLoginInfoRequest,
		GetUserLoginInfoResponse,
		ListUserLoginsRequest,
		ListUserLoginsResponse,
		UserWebLogin,
	},
	utils::{DateTime, DecodedRequest, Location},
};
use axum::{extract::State, Extension, Router};
use chrono::{Duration, Utc};

use crate::{
	app::App,
	db,
	models::UserAuthenticationData,
	prelude::*,
	redis,
	service::get_access_token_expiry,
	utils::Error,
};

pub fn create_sub_app(app: &App) -> Router<App> {
	Router::new()
		.mount_protected_dto(
			PlainTokenAuthenticator::new().disallow_api_token(),
			app.clone(),
			get_all_logins_for_user,
		)
		.mount_protected_dto(
			PlainTokenAuthenticator::new().disallow_api_token(),
			app.clone(),
			get_login_info,
		)
		.mount_protected_dto(
			PlainTokenAuthenticator::new().disallow_api_token(),
			app.clone(),
			delete_user_login,
		)
}

async fn get_all_logins_for_user(
	mut connection: Connection,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: _,
		query: _,
		body: _,
	}: DecodedRequest<ListUserLoginsRequest>,
) -> Result<ListUserLoginsResponse, Error> {
	let user_id = token_data.user_id();

	let logins = db::get_all_web_logins_for_user(&mut connection, &user_id)
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

	Ok(ListUserLoginsResponse { logins })
}

async fn get_login_info(
	mut connection: Connection,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path,
		query: _,
		body: _,
	}: DecodedRequest<GetUserLoginInfoRequest>,
) -> Result<GetUserLoginInfoResponse, Error> {
	let login = db::get_user_web_login(&mut connection, &path.login_id)
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
		.ok_or_else(|| ErrorType::WrongParameters)?;

	Ok(GetUserLoginInfoResponse { login })
}

async fn delete_user_login(
	mut connection: Connection,
	Extension(token_data): Extension<UserAuthenticationData>,
	State(mut app): State<App>,
	DecodedRequest {
		path,
		query: _,
		body: _,
	}: DecodedRequest<DeleteUserLoginRequest>,
) -> Result<(), Error> {
	let user_id = token_data.user_id();

	db::delete_user_web_login_by_id(&mut connection, &path.login_id, &user_id)
		.await?;

	let ttl = get_access_token_expiry() + Duration::hours(2); // 2 hrs buffer time
	redis::revoke_login_tokens_created_before_timestamp(
		&mut app.redis,
		&path.login_id,
		&Utc::now(),
		Some(&ttl),
	)
	.await?;

	Ok(())
}
