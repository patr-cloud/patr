use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use http::StatusCode;
use models::api::auth::oauth::*;
use rustis::commands::{GenericCommands, StringCommands};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuthCodeData {
	code_challenge: String,
	code_challenge_method: CodeChallengeHashMethod,
}

pub async fn token(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: OAuthTokenPath,
				query: (),
				headers: (),
				body:
					OAuthTokenRequestProcessed {
						grant_type,
						client_id,
						code,
						redirect_uri,
						refresh_token,
						code_verifier,
					},
			},
		database,
		redis,
		client_ip,
		config,
	}: AppRequest<'_, OAuthTokenRequest>,
) -> Result<AppResponse<OAuthTokenRequest>, ErrorType> {
	if !matches!(
		grant_type,
		OAuthTokenGrantType::AuthorizationCode | OAuthTokenGrantType::RefreshToken
	) {
		return Err(ErrorType::OAuthInvalidGrantType);
	}
	if grant_type == OAuthTokenGrantType::AuthorizationCode {
		// Handle authorization code grant type
		let key = redis::keys::oauth_authorization_code_prefix(&code);
		let auth_code_string: Option<String> = redis.get(&key).await.inspect_err(|err| {
			error!(
				"Error retrieving authorization code from Redis: {}",
				err.to_string()
			);
		})?;
		_ = redis.del(&key).await;
		if auth_code_string.is_none() {
			return Err(ErrorType::OAuthInvalidAuthorizationCode);
		}
		let auth_code_data: AuthCodeData = serde_json::from_str(&auth_code_string.unwrap())
			.map_err(|_| ErrorType::OAuthInvalidAuthorizationCode)?;
		if auth_code_data.code_challenge_method == CodeChallengeHashMethod::SHA256 {
			let result = Sha256::digest(code_verifier.as_bytes());
			let code_verifier_hashed = URL_SAFE_NO_PAD.encode(&result);
			if code_verifier_hashed != auth_code_data.code_challenge {
				return Err(ErrorType::OAuthInvalidAuthorizationCode);
			}
		} else {
			if code_verifier != auth_code_data.code_challenge {
				return Err(ErrorType::OAuthInvalidAuthorizationCode);
			}
		}
	} else if grant_type == OAuthTokenGrantType::RefreshToken {
		// Handle refresh token grant type
		let key = redis::keys::oauth_refresh_token_prefix(&refresh_token.unwrap());
		let user_id: Option<String> = redis.get(&key).await.inspect_err(|err| {
			error!(
				"Error retrieving refresh token from Redis: {}",
				err.to_string()
			);
		})?;
		let _ = redis.del(&key).await;
		if user_id.is_none() {
			return Err(ErrorType::OAuthInvalidRefreshToken);
		}
	}
	let access_token = Uuid::new_v4().to_string();
	let refresh_token = Uuid::new_v4().to_string();
	redis
		.setex(
			redis::keys::oauth_refresh_token_prefix(&refresh_token),
			3600,
			"some_user_id",
		)
		.await
		.inspect_err(|err| {
			error!("Error storing refresh token in Redis: {}", err.to_string());
		})?;
	let response = OAuthTokenResponse {
		access_token,
		token_type: "Bearer".into(),
		expires_in: 3600,
		refresh_token,
		scope: "read write".into(),
	};

	return AppResponse::builder()
		.body(response)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result();
}
