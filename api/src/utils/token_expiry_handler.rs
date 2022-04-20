use api_models::utils::Uuid;
use eve_rs::AsError;
use redis::{
	aio::MultiplexedConnection as RedisConnection,
	AsyncCommands,
	RedisError,
};

use super::{get_current_time_millis, Error};
use crate::{error, models::AccessTokenData, service::get_access_token_expiry};

const GLOBAL_USER_EXP: &str = "token:global-user-exp";
const USER_ID_EXP: &str = "token:user-{}-exp";
const LOGIN_ID_EXP: &str = "token:login-{}-exp";

pub struct TokenExpiryHandler {
	conn: RedisConnection,
}

impl TokenExpiryHandler {
	pub fn new(redis_conn: RedisConnection) -> Self {
		TokenExpiryHandler { conn: redis_conn }
	}

	// TODO: better error handling
	// TODO: better naming
	pub async fn validate_access_token(
		&mut self,
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
		let expiry: Option<u64> = self.conn.get(&user_id).await?;
		if expiry.map_or(false, |exp| token.iat < exp) {
			return Error::as_result()
				.status(401)
				.body(error!(UNAUTHORIZED).to_string())?;
		}

		// 3. check whether token has been expired due to expired login id
		let login_id = LOGIN_ID_EXP.replace("{}", token.login_id.as_str());
		let expiry: Option<u64> = self.conn.get(&login_id).await?;
		if expiry.map_or(false, |exp| token.iat < exp) {
			return Error::as_result()
				.status(401)
				.body(error!(UNAUTHORIZED).to_string())?;
		}

		// 4. check whether token has been expired due to expired jwt key
		let global_expiry: Option<u64> = self.conn.get(GLOBAL_USER_EXP).await?;
		if global_expiry.map_or(false, |global_exp| token.iat < global_exp) {
			return Error::as_result()
				.status(401)
				.body(error!(UNAUTHORIZED).to_string())?;
		}

		// all checks are passed, hence a valid token
		Ok(())
	}

	pub async fn expire_tokens_for_user_id(
		&mut self,
		user_id: &Uuid,
	) -> Result<(), RedisError> {
		let key = USER_ID_EXP.replace("{}", user_id.as_str());
		let ttl = (get_access_token_expiry() / 60) as usize + 60; // 60 sec buffer time
		self.conn
			.set_ex(key, get_current_time_millis(), ttl)
			.await?;
		Ok(())
	}

	pub async fn _expire_token_for_login_id(
		&mut self,
		login_id: &Uuid,
	) -> Result<(), RedisError> {
		let key = LOGIN_ID_EXP.replace("{}", login_id.as_str());
		let ttl = (get_access_token_expiry() / 60) as usize + 60; // 60 sec buffer time
		self.conn
			.set_ex(key, get_current_time_millis(), ttl)
			.await?;
		Ok(())
	}

	pub async fn _expire_all_tokens(&mut self) -> Result<(), RedisError> {
		let ttl = (get_access_token_expiry() / 60) as usize + 60; // 60 sec buffer time
		self.conn
			.set_ex(GLOBAL_USER_EXP, get_current_time_millis(), ttl)
			.await?;
		Ok(())
	}
}
