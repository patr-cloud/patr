use api_models::utils::Uuid;
use eve_rs::AsError;
use redis::{
	aio::MultiplexedConnection as RedisConnection,
	AsyncCommands,
	RedisError,
};

use crate::{
	error,
	models::AccessTokenData,
	service::get_access_token_expiry,
	utils::{get_current_time_millis, Error},
};

const GLOBAL_USER_EXP: &str = "token:global-user-exp";
const USER_ID_EXP: &str = "token:user-{}-exp";
const LOGIN_ID_EXP: &str = "token:login-{}-exp";
const WORKSPACE_ID_EXP: &str = "token:workspace-{}-exp";

pub async fn validate_access_token(
	conn: &mut RedisConnection,
	token: &AccessTokenData,
) -> Result<(), Error> {
	// 1. check whether token expired naturally
	if token.exp < get_current_time_millis() {
		return Error::as_result()
			.status(401)
			.body(error!(EXPIRED).to_string())?;
	}

	// 2. check whether token has been expired due to expired user id
	let user_id = USER_ID_EXP.replace("{}", token.user.id.as_str());
	let expiry: Option<u64> = conn.get(&user_id).await?;
	if expiry.map_or(false, |exp| token.iat < exp) {
		return Error::as_result()
			.status(401)
			.body(error!(UNAUTHORIZED).to_string())?;
	}

	// 3. check whether token has been expired due to expired login id
	let login_id = LOGIN_ID_EXP.replace("{}", token.login_id.as_str());
	let expiry: Option<u64> = conn.get(&login_id).await?;
	if expiry.map_or(false, |exp| token.iat < exp) {
		return Error::as_result()
			.status(401)
			.body(error!(UNAUTHORIZED).to_string())?;
	}

	// 4. check whether token has been expired due to expired workspace id
	for workspace_id in token.workspaces.keys() {
		let workspace_id =
			WORKSPACE_ID_EXP.replace("{}", workspace_id.as_str());
		let expiry: Option<u64> = conn.get(&workspace_id).await?;
		if expiry.map_or(false, |exp| token.iat < exp) {
			return Error::as_result()
				.status(401)
				.body(error!(UNAUTHORIZED).to_string())?;
		}
	}

	// 5. check whether token has been expired due to expired jwt key
	let global_expiry: Option<u64> = conn.get(GLOBAL_USER_EXP).await?;
	if global_expiry.map_or(false, |global_exp| token.iat < global_exp) {
		return Error::as_result()
			.status(401)
			.body(error!(UNAUTHORIZED).to_string())?;
	}

	// all checks are passed, hence a valid token
	Ok(())
}

pub async fn expire_tokens_for_user_id(
	conn: &mut RedisConnection,
	user_id: &Uuid,
) -> Result<(), RedisError> {
	let key = USER_ID_EXP.replace("{}", user_id.as_str());
	set_as_expired(conn, &key).await
}

pub async fn expire_token_for_login_id(
	conn: &mut RedisConnection,
	login_id: &Uuid,
) -> Result<(), RedisError> {
	let key = LOGIN_ID_EXP.replace("{}", login_id.as_str());
	set_as_expired(conn, &key).await
}

pub async fn expire_tokens_for_workspace_id(
	conn: &mut RedisConnection,
	workspace_id: &Uuid,
) -> Result<(), RedisError> {
	let key = WORKSPACE_ID_EXP.replace("{}", workspace_id.as_str());
	set_as_expired(conn, &key).await
}

pub async fn expire_all_tokens(
	conn: &mut RedisConnection,
) -> Result<(), RedisError> {
	set_as_expired(conn, GLOBAL_USER_EXP).await
}

async fn set_as_expired(
	conn: &mut RedisConnection,
	key: &str,
) -> Result<(), RedisError> {
	let ttl = (get_access_token_expiry() / 60) as usize + 60; // 60 sec buffer time
	conn.set_ex(key, get_current_time_millis(), ttl).await
}
