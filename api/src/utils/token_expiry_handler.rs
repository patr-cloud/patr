use api_models::utils::Uuid;
use eve_rs::AsError;
use redis::aio::MultiplexedConnection as RedisConnection;
use redis::{AsyncCommands, RedisError};

use crate::error;
use crate::models::AccessTokenData;
use crate::service::get_access_token_expiry;

use super::{get_current_time_millis, Error};

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
		let user_id_key = USER_ID_EXP.replace("{}", token.user.id.as_str());
		let user_id_expiry: Option<u64> = self.conn.get(&user_id_key).await?;
		if user_id_expiry.map_or(false, |exp| token.exp < exp) {
			return Error::as_result()
				.status(401)
				.body(error!(UNAUTHORIZED).to_string())?;
		}

		// 3. check whether token has been expired due to expired login id
		let login_id_key = LOGIN_ID_EXP.replace("{}", token.login_id.as_str());
		let login_id_expiry: Option<u64> = self.conn.get(&login_id_key).await?;
		if login_id_expiry.map_or(false, |exp| token.exp < exp) {
			return Error::as_result()
				.status(401)
				.body(error!(UNAUTHORIZED).to_string())?;
		}

		// 4. check whether token has been expired due to expired jwt key
		let global_expiry: Option<u64> = self.conn.get(GLOBAL_USER_EXP).await?;
		if global_expiry.map_or(false, |global_exp| token.exp < global_exp) {
			return Error::as_result()
				.status(401)
				.body(error!(UNAUTHORIZED).to_string())?;
		}

		// all checks are passed, hence a valid token
		Ok(())
	}

	pub async fn expire_tokens_for_user_id(
		&mut self,
		user_id: Uuid,
	) -> Result<(), RedisError> {
		let key = USER_ID_EXP.replace("{}", user_id.as_str());
		self.conn
			.set_ex(key, get_current_time_millis(), Self::get_ttl())
			.await?;
		Ok(())
	}

	pub async fn expire_token_for_login_id(
		&mut self,
		login_id: Uuid,
	) -> Result<(), RedisError> {
		let key = LOGIN_ID_EXP.replace("{}", login_id.as_str());
		self.conn
			.set_ex(key, get_current_time_millis(), Self::get_ttl())
			.await?;
		Ok(())
	}

	pub async fn expire_all_tokens(&mut self) -> Result<(), RedisError> {
		self.conn
			.set_ex(GLOBAL_USER_EXP, get_current_time_millis(), Self::get_ttl())
			.await?;
		Ok(())
	}

	fn get_ttl() -> usize {
		// access token expiry time + 1 hours addtional buffer time
		get_access_token_expiry() as usize + (1000 * 60 * 60)
	}
}
