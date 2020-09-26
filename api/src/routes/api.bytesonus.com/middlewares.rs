use crate::models::{error, AccessTokenData};
use crate::utils::{constants::request_keys, MiddlewareHandlerFunction};

use eve_rs::Context;
use serde_json::json;

pub fn token_authenticator() -> MiddlewareHandlerFunction {
	|mut context, next| {
		Box::pin(async move {
			if let Some(header) = context.get_header("Authorization") {
				if let Ok(token_data) = AccessTokenData::parse(
					header,
					&context.get_state().config.jwt_secret,
				) {
					context.set_token_data(token_data);
					next(context).await
				} else {
					context.status(401).json(json!({
						request_keys::SUCCESS: false,
						request_keys::ERROR: error::id::UNAUTHORIZED,
						request_keys::MESSAGE: error::message::UNAUTHORIZED,
					}));
					Ok(context)
				}
			} else {
				context.status(401).json(json!({
					request_keys::SUCCESS: false,
					request_keys::ERROR: error::id::UNAUTHORIZED,
					request_keys::MESSAGE: error::message::UNAUTHORIZED,
				}));
				Ok(context)
			}
		})
	}
}
