use axum::http::StatusCode;
use models::api::auth::*;
use rustis::commands::GenericCommands;
use sha2::{Digest as _, Sha256};

use crate::{models::permissions, prelude::*, redis::keys as redis};

pub async fn logout(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: LogoutPath,
				query: (),
				headers: LogoutRequestHeaders {
					refresh_token,
					user_agent: _,
				},
				body: LogoutRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		actor_data,
		state: _,
	}: AuthenticatedAppRequest<'_, LogoutRequest>,
) -> Result<AppResponse<LogoutRequest>, ErrorType> {
	info!("Logging out user: {}", actor_data.id);

	// User agent being a browser is expected to be checked in the
	// UserAgentValidationLayer

	let refresh_token = refresh_token.0.token();
	if !refresh_token.starts_with(constants::REFRESH_TOKEN_PREFIX) ||
		!permissions::has_valid_checksum(refresh_token)
	{
		return Err(ErrorType::MalformedRefreshToken);
	}

	let Some(login) = query!(
		r#"
		SELECT
			login_id
		FROM
			web_login
		WHERE
			token_hash = $1;
		"#,
		hex::encode(Sha256::digest(refresh_token)),
	)
	.fetch_optional(&mut **database)
	.await?
	else {
		debug!("Could not find a login for that refresh token");
		return Err(ErrorType::MalformedRefreshToken);
	};
	let login_id = login.login_id;

	query!(
		r#"
		DELETE FROM
			web_login
		WHERE
			login_id = $1;
		"#,
		login_id,
	)
	.execute(&mut **database)
	.await?;

	trace!("Deleted web login");

	query!(
		r#"
		DELETE FROM
			user_login
		WHERE
			login_id = $1;
		"#,
		login_id,
	)
	.execute(&mut **database)
	.await?;

	trace!("Deleted user login");

	_ = redis
		.del(redis::auth_data_for_login_id(&login_id))
		.await
		.inspect_err(|err| {
			error!(
				"Error deleting the cached permission for login `{}`: `{}`",
				login_id, err
			);
		});
	permissions::mark_login_stale(redis, &login_id).await?;

	AppResponse::builder()
		.body(LogoutResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
