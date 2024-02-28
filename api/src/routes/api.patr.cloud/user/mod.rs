use argon2::{Algorithm, PasswordHash, PasswordVerifier, Version};
use axum::{http::StatusCode, Router};
use models::{
	api::{user::*, workspace::Workspace, WithId},
	ErrorType,
};
use rustis::commands::StringCommands;
use sqlx::query;
use time::Duration;
use totp_rs::{Algorithm as TotpAlgorithm, Secret, TOTP};

use crate::{prelude::*, redis::keys::get_key_for_user_mfa_secret};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_auth_endpoint(change_password, state)
		.mount_auth_endpoint(get_user_details, state)
		.mount_auth_endpoint(get_user_info, state)
		.mount_auth_endpoint(list_workspaces, state)
		.mount_auth_endpoint(set_user_info, state)
		.mount_auth_endpoint(activate_mfa, state)
		.mount_auth_endpoint(deactivate_mfa, state)
		.mount_auth_endpoint(get_mfa_secret, state)
		.with_state(state.clone())
}

async fn change_password(
	AuthenticatedAppRequest {
		request: ProcessedApiRequest {
			path: _,
			query: _,
			headers: _,
			body,
		},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data: _,
	}: AuthenticatedAppRequest<'_, ChangePasswordRequest>,
) -> Result<AppResponse<ChangePasswordRequest>, ErrorType> {
	info!("Starting: Change password");

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
		""
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::UserNotFound)?;

	let success = argon2::Argon2::new_with_secret(
		config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.map_err(|err| ErrorType::server_error(err.to_string()))?
	.verify_password(
		body.current_password.as_ref(),
		&PasswordHash::new(&user_data.password)
			.map_err(|err| ErrorType::server_error(err.to_string()))?,
	)
	.is_ok();

	if !success {
		return Err(ErrorType::InvalidPassword);
	}

	if let Some(mfa_secret) = user_data.mfa_secret {
		let Some(mfa_otp) = body.mfa_otp else {
			return Err(ErrorType::MfaRequired);
		};

		let totp = TOTP::new(
			TotpAlgorithm::SHA1,
			6,
			1,
			30,
			Secret::Encoded(mfa_secret).to_bytes().map_err(|err| {
				error!(
					"Unable to parse MFA secret for userId `{}`: {}",
					user_data.id,
					err.to_string()
				);
				ErrorType::server_error(err.to_string())
			})?,
		)
		.map_err(|err| {
			error!(
				"Unable to parse TOTP for userId `{}`: {}",
				user_data.id,
				err.to_string()
			);
			ErrorType::server_error(err.to_string())
		})?;

		let mfa_valid = totp.check_current(&mfa_otp).map_err(|err| {
			error!(
				"System time error while checking TOTP for userId `{}`: {}",
				user_data.id,
				err.to_string()
			);
			ErrorType::server_error(err.to_string())
		})?;

		if !mfa_valid {
			return Err(ErrorType::MfaOtpInvalid);
		}
	}

	query!(
		r#"
		UPDATE
			"user"
		SET
			password = $1
		WHERE
			id = $2;
		"#,
		body.new_password,
		user_data.id as _,
	)
	.execute(&mut **database)
	.await?;

	AppResponse::builder()
		.body(ChangePasswordResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn get_user_details(
	AuthenticatedAppRequest {
		request: ProcessedApiRequest {
			path,
			query: _,
			headers: _,
			body: _,
		},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data,
	}: AuthenticatedAppRequest<'_, GetUserDetailsRequest>,
) -> Result<AppResponse<GetUserDetailsRequest>, ErrorType> {
	info!("Starting: Get user details by UserID");

	let basic_user_info = query!(
		r#"
		SELECT
			"user".username,
			"user".first_name,
			"user".last_name
		FROM
			"user"
		WHERE
			id = $1;
		"#,
		path.user_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.map(|row| GetUserDetailsResponse {
		basic_user_info: WithId::new(
			user_data.id,
			BasicUserInfo {
				username: row.username,
				first_name: row.first_name,
				last_name: row.last_name,
			},
		),
	})
	.ok_or(ErrorType::UserNotFound)?;

	AppResponse::builder()
		.body(basic_user_info)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn get_user_info(
	AuthenticatedAppRequest {
		request: ProcessedApiRequest {
			path: _,
			query: _,
			headers: _,
			body: _,
		},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data,
	}: AuthenticatedAppRequest<'_, GetUserInfoRequest>,
) -> Result<AppResponse<GetUserInfoRequest>, ErrorType> {
	info!("Starting: Get authenticated user info");

	let user_info = query!(
		r#"
		SELECT
			"user".username,
			"user".first_name,
			"user".last_name,
			"user".created,
			"user".mfa_secret,
			"user".recovery_phone_country_code,
			"user".recovery_phone_number,
			"user".recovery_email
		FROM
			"user"
		WHERE
			"user".id = $1;
		"#,
		user_data.id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.map(|row| GetUserInfoResponse {
		basic_user_info: WithId::new(
			user_data.id,
			BasicUserInfo {
				username: row.username,
				first_name: row.first_name,
				last_name: row.last_name,
			},
		),
		created: row.created,
		is_mfa_enabled: row.mfa_secret.is_some(),
		recovery_email: row.recovery_email,
		recovery_phone_number: row
			.recovery_phone_country_code
			.zip(row.recovery_phone_number)
			.map(|(country_code, phone_number)| UserPhoneNumber {
				country_code,
				phone_number,
			}),
	})
	.ok_or(ErrorType::UserNotFound)?;

	AppResponse::builder()
		.body(user_info)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn list_workspaces(
	AuthenticatedAppRequest {
		request: ProcessedApiRequest {
			path: _,
			query: _,
			headers: _,
			body: _,
		},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data,
	}: AuthenticatedAppRequest<'_, ListUserWorkspacesRequest>,
) -> Result<AppResponse<ListUserWorkspacesRequest>, ErrorType> {
	info!("Starting: List all user workspaces");

	let list_of_workspaces = query!(
		r#"
		SELECT DISTINCT
			workspace.id as "id: Uuid",
			workspace.name::TEXT as "name!: String",
			workspace.super_admin_id as "super_admin_id: Uuid"
		FROM
			workspace
		LEFT JOIN
			workspace_user
		ON
			workspace.id = workspace_user.workspace_id
		WHERE
			(
				workspace.super_admin_id = $1 OR
				workspace_user.user_id = $1
			) AND
			workspace.deleted IS NULL;
		"#,
		user_data.id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		WithId::new(
			user_data.id,
			Workspace {
				name: row.name,
				super_admin_id: row.super_admin_id,
			},
		)
	})
	.collect();

	AppResponse::builder()
		.body(ListUserWorkspacesResponse {
			workspaces: list_of_workspaces,
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn set_user_info(
	AuthenticatedAppRequest {
		request: ProcessedApiRequest {
			path: _,
			query: _,
			headers: _,
			body,
		},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data,
	}: AuthenticatedAppRequest<'_, UpdateUserInfoRequest>,
) -> Result<AppResponse<UpdateUserInfoRequest>, ErrorType> {
	info!("Starting: Update user info");

	query!(
		r#"
		UPDATE
			"user"
		SET
			first_name = COALESCE($1, first_name),
			last_name = COALESCE($2, last_name)
		WHERE
			id = $3;
		"#,
		body.first_name,
		body.last_name,
		user_data.id as _,
	)
	.execute(&mut **database)
	.await?;

	AppResponse::builder()
		.body(UpdateUserInfoResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn activate_mfa(
	AuthenticatedAppRequest {
		request: ProcessedApiRequest {
			path: _,
			query: _,
			headers: _,
			body,
		},
		database,
		redis,
		client_ip: _,
		config: _,
		user_data,
	}: AuthenticatedAppRequest<'_, ActivateMfaRequest>,
) -> Result<AppResponse<ActivateMfaRequest>, ErrorType> {
	info!("Starting: Activate MFA for user");

	let mfa_detail = query!(
		r#"
		SELECT
			"user".mfa_secret
		FROM
			"user"
		WHERE
			id = $1;
		"#,
		user_data.id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::UserNotFound)?;

	if mfa_detail.mfa_secret.is_some() {
		return Err(ErrorType::MfaAlreadyActive);
	}

	let secret: String = redis
		.get(get_key_for_user_mfa_secret(&user_data.id))
		.await?;

	let totp = TOTP::new(
		TotpAlgorithm::SHA1,
		6,
		1,
		30,
		Secret::Encoded(secret.clone())
			.to_bytes()
			.inspect_err(|err| {
				error!(
					"Unable to parse MFA secret for userId `{}`: {}",
					user_data.id,
					err.to_string()
				);
			})
			.map_err(ErrorType::server_error)?,
	)
	.inspect_err(|err| {
		error!(
			"Unable to parse TOTP for userId `{}`: {}",
			user_data.id,
			err.to_string()
		);
	})
	.map_err(ErrorType::server_error)?;

	if !(totp.check_current(&body.otp)?) {
		return Err(ErrorType::MfaOtpInvalid);
	}

	query!(
		r#"
		UPDATE
			"user"
		SET
			mfa_secret = $2
		WHERE
			id = $1;
		"#,
		user_data.id as _,
		secret
	)
	.execute(&mut **database)
	.await?;

	AppResponse::builder()
		.body(ActivateMfaResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn deactivate_mfa(
	AuthenticatedAppRequest {
		request: ProcessedApiRequest {
			path: _,
			query: _,
			headers: _,
			body,
		},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data,
	}: AuthenticatedAppRequest<'_, DeactivateMfaRequest>,
) -> Result<AppResponse<DeactivateMfaRequest>, ErrorType> {
	info!("Starting: Deactivate MFA for user");

	let mfa_detail = query!(
		r#"
		SELECT
			"user".mfa_secret
		FROM
			"user"
		WHERE
			id = $1;
		"#,
		user_data.id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::UserNotFound)?;

	let Some(secret) = mfa_detail.mfa_secret else {
		return Err(ErrorType::MfaAlreadyInactive);
	};

	let totp = TOTP::new(
		TotpAlgorithm::SHA1,
		6,
		1,
		30,
		Secret::Encoded(secret.clone())
			.to_bytes()
			.inspect_err(|err| {
				error!(
					"Unable to parse MFA secret for userId `{}`: {}",
					user_data.id,
					err.to_string()
				);
			})
			.map_err(ErrorType::server_error)?,
	)
	.inspect_err(|err| {
		error!(
			"Unable to parse TOTP for userId `{}`: {}",
			user_data.id,
			err.to_string()
		);
	})
	.map_err(ErrorType::server_error)?;

	if !(totp.check_current(&body.otp)?) {
		return Err(ErrorType::MfaOtpInvalid);
	}

	query!(
		r#"
		UPDATE
			"user"
		SET
			mfa_secret = NULL
		WHERE
			id = $1;
		"#,
		user_data.id as _
	)
	.execute(&mut **database)
	.await?;

	AppResponse::builder()
		.body(DeactivateMfaResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn get_mfa_secret(
	AuthenticatedAppRequest {
		request: ProcessedApiRequest {
			path: _,
			query: _,
			headers: _,
			body: _,
		},
		database,
		redis,
		client_ip: _,
		config: _,
		user_data,
	}: AuthenticatedAppRequest<'_, GetMfaSecretRequest>,
) -> Result<AppResponse<GetMfaSecretRequest>, ErrorType> {
	info!("Starting: Get MFA secret");

	let mfa_detail = query!(
		r#"
		SELECT
			"user".mfa_secret
		FROM
			"user"
		WHERE
			id = $1;
		"#,
		user_data.id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::UserNotFound)?;

	if mfa_detail.mfa_secret.is_some() {
		return Err(ErrorType::MfaAlreadyActive);
	}

	let secret = Secret::generate_secret().to_encoded().to_string();

	redis
		.setex(
			get_key_for_user_mfa_secret(&user_data.id),
			Duration::minutes(5).whole_seconds() as u64,
			secret.clone(),
		)
		.await?;

	AppResponse::builder()
		.body(GetMfaSecretResponse { secret })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
