use argon2::{Algorithm, PasswordHash, PasswordVerifier, Version};
use axum::http::StatusCode;
use models::api::auth::oauth::*;
use rustis::commands::StringCommands;
use serde_json::json;
use totp_rs::{Algorithm as TotpAlgorithm, Secret, TOTP};

use crate::prelude::*;

pub async fn authorize(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: OAuthAuthorizePath,
				query:
					OAuthAuthorizeQuery {
						response_type,
						client_id,
						redirect_uri,
						scope,
						state,
						code_challenge,
						code_challenge_method,
					},
				headers: (),
				body: OAuthAuthorizeRequestProcessed,
			},
		database,
		redis: _,
		client_ip,
		config,
	}: AppRequest<'_, OAuthAuthorizeRequest>,
) -> Result<AppResponse<OAuthAuthorizeRequest>, ErrorType> {
	if response_type != OAuthAuthorizeResponseType::AuthorizationCode {
		return Err(ErrorType::InvalidResponseType);
	}

	let redirect_uri_str = redirect_uri.as_deref().unwrap_or("");
	let state_str = state.as_deref().unwrap_or("");
	let code_challenge_method_str = match code_challenge_method {
		CodeChallengeHashMethod::SHA256 => "S256",
		CodeChallengeHashMethod::Plain => "plain",
	};

	let login = format!(
		"http://localhost:3001/oauth/authorize?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method={}",
		client_id,
		urlencoding::encode(redirect_uri_str),
		urlencoding::encode(&scope),
		urlencoding::encode(state_str),
		urlencoding::encode(&code_challenge),
		code_challenge_method_str
	);

	AppResponse::builder()
		.body(OAuthAuthorizeResponse)
		.headers(OAuthAuthorizeResponseHeaders {
			redirect_url: login.parse().unwrap(),
		})
		.status_code(StatusCode::FOUND)
		.build()
		.into_result()
}

pub async fn authorize_post(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: OAuthAuthorizePostPath,
				query: (),
				headers: (),
				body:
					OAuthAuthorizePostRequestProcessed {
						user_id,
						password,
						mfa_otp,
						client_id,
						redirect_uri,
						scope,
						state,
						code_challenge,
						code_challenge_method,
					},
			},
		database,
		redis,
		client_ip,
		config,
	}: AppRequest<'_, OAuthAuthorizePostRequest>,
) -> Result<AppResponse<OAuthAuthorizePostRequest>, ErrorType> {
	let user_data = query!(
		r#"
		SELECT
			"user".id,
			"user".username,
			"user".password,
			"user".mfa_secret
		FROM
			"user"
		LEFT JOIN
			user_email
		ON
			user_email.user_id = "user".id
		LEFT JOIN
			user_phone_number
		ON
			user_phone_number.user_id = "user".id
		LEFT JOIN
			phone_number_country_code
		ON
			phone_number_country_code.country_code = user_phone_number.country_code
		WHERE
			"user".username = $1 OR
			user_email.email = $1 OR
			CONCAT(
				'+',
				phone_number_country_code.phone_code,
				user_phone_number.number
			) = $1;
		"#,
		&user_id,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::UserNotFound)?;

	trace!("Found user with ID: {}", user_data.id);

	let success = argon2::Argon2::new_with_secret(
		config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.inspect_err(|err| {
		error!("Error creating Argon2: `{}`", err);
	})
	.map_err(ErrorType::server_error)?
	.verify_password(
		password.as_bytes(),
		&PasswordHash::new(&user_data.password).map_err(ErrorType::server_error)?,
	)
	.inspect_err(|err| {
		info!("Error verifying password: `{}`", err);
	})
	.is_ok();

	if !success {
		return Err(ErrorType::InvalidPassword);
	}

	trace!("Password hashes match");

	if let Some(mfa_secret) = user_data.mfa_secret {
		trace!("User has MFA secret");

		let Some(mfa_otp) = mfa_otp else {
			return Err(ErrorType::MfaRequired);
		};

		let totp = TOTP::new(
			TotpAlgorithm::SHA1,
			6,
			1,
			30,
			Secret::Encoded(mfa_secret).to_bytes().inspect_err(|err| {
				error!(
					"Unable to parse MFA secret for userId `{}`: {}",
					user_data.id,
					err.to_string()
				);
			})?,
		)
		.inspect_err(|err| {
			error!(
				"Unable to parse TOTP for userId `{}`: {}",
				user_data.id,
				err.to_string()
			);
		})?;

		let mfa_valid = totp.check_current(&mfa_otp).inspect_err(|err| {
			error!(
				"System time error while checking TOTP for userId `{}`: {}",
				user_data.id,
				err.to_string()
			);
		})?;

		if !mfa_valid {
			return Err(ErrorType::MfaOtpInvalid);
		}

		trace!("User MFA is valid");
	}

	let authorization_code = Uuid::new_v4().to_string();
	info!(
		"Generated authorization code `{}` for client_id `{}`",
		authorization_code, client_id
	);
	let metadata = json!({
		"code_challenge": code_challenge,
		"code_challenge_method": code_challenge_method,
	});

	// Store the authorization code and its metadata in the redis with an expiration
	// time.
	let key = format!("auth_code:{}", authorization_code);
	let exp_time = 600;
	redis
		.setex(key, exp_time, metadata.to_string())
		.await
		.inspect_err(|err| {
			error!(
				"Error storing authorization code in Redis for userId : {}",
				// user_data.id,
				err.to_string()
			);
		})
		.map_err(ErrorType::server_error)?;

	let redirect_uri_str = redirect_uri.as_deref().unwrap_or("");
	let state_str = state.as_deref().unwrap_or("");

	let redirect_url = format!(
		"{}?code={}&state={}",
		redirect_uri_str,
		urlencoding::encode(&authorization_code),
		urlencoding::encode(state_str)
	);

	AppResponse::builder()
		.body(OAuthAuthorizePostResponse)
		.headers(OAuthAuthorizePostResponseHeaders {
			redirect_url: redirect_url.parse().unwrap(),
		})
		.status_code(StatusCode::FOUND)
		.build()
		.into_result()
}
