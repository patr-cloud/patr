use axum::http::StatusCode;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use jsonwebtoken::{DecodingKey, Validation, decode};
use models::api::auth::oauth::*;
use sha2::{Digest, Sha256};

use crate::{
	prelude::*,
	routes::api_patr_cloud::auth::oauth::authorize::{AuthClaims, create_token},
};

pub fn decode_token(token: &str, secret: &str) -> Result<AuthClaims, jsonwebtoken::errors::Error> {
	info!("Decoding token: {}", token);
	match decode::<AuthClaims>(
		token,
		&DecodingKey::from_secret(secret.as_ref()),
		&Validation::default(),
	) {
		Ok(token_data) => {
			info!("Decoded token data: {:?}", token_data);
			Ok(token_data.claims)
		}
		Err(err) => {
			error!("Failed to decode token: {:?}", err);
			Err(err)
		}
	}
}

pub fn verify_pkce(code_verifier: &str, code_challenge: String) -> bool {
	let hash = Sha256::digest(code_verifier.as_bytes());
	info!("Computed hash: {:?}", hash);
	let encoded = STANDARD.encode(hash);
	info!("Encoded hash: {}", encoded);
	encoded == code_challenge
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
		redis: _,
		client_ip,
		config,
	}: AppRequest<'_, OAuthTokenRequest>,
) -> Result<AppResponse<OAuthTokenRequest>, ErrorType> {
	if !matches!(
		grant_type,
		OAuthTokenGrantType::AuthorizationCode | OAuthTokenGrantType::RefreshToken
	) {
		return Err(ErrorType::AuthorizationTokenInvalid);
	}
	if let OAuthTokenGrantType::AuthorizationCode = grant_type {
		let token_data = decode_token(&code, &config.jwt_secret)
			.map_err(|_| ErrorType::AuthorizationTokenInvalid)?;

		// Verify PKCE
		info!(
			"Verifying PKCE with code_verifier: {:?}",
			token_data.code_challenge
		);
		if !verify_pkce(&code_verifier, token_data.code_challenge) {
			return Err(ErrorType::AuthorizationTokenInvalid);
		}

		// PKCE is valid, issue access + refresh tokens
		let access_token = create_token("user_id_or_scope".to_string(), &config.jwt_secret);
		let refresh_token = create_token("refresh_user_id".to_string(), &config.jwt_secret);

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
	} else {
		// Verify the refresh token is provided
		let refresh_token_str = refresh_token
			.as_ref()
			.ok_or(ErrorType::AuthorizationTokenInvalid)?;

		let token_data = decode_token(refresh_token_str, &config.jwt_secret)
			.map_err(|_| ErrorType::AuthorizationTokenInvalid)?;

		info!(
			"Refresh token decoded successfully for user: {:?}",
			token_data
		);

		// Issue new access token and refresh token
		let new_access_token = create_token("user_id_or_scope".to_string(), &config.jwt_secret);
		let new_refresh_token = create_token("refresh_user_id".to_string(), &config.jwt_secret);

		let response = OAuthTokenResponse {
			access_token: new_access_token,
			token_type: "Bearer".into(),
			expires_in: 3600,
			refresh_token: new_refresh_token,
			scope: "read write".into(),
		};

		return AppResponse::builder()
			.body(response)
			.headers(())
			.status_code(StatusCode::OK)
			.build()
			.into_result();
	}
}
