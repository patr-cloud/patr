use axum::http::StatusCode;
use models::api::user::*;

use crate::{models::permissions, prelude::*};

/// Delete one of the user's web logins, logging that session out. Deleting
/// the login the request itself came from is allowed and behaves like logout.
pub async fn delete_web_login(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: DeleteWebLoginPath { login_id },
				query: (),
				headers:
					DeleteWebLoginRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: DeleteWebLoginRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		user_data,
		state: _,
	}: AuthenticatedAppRequest<'_, DeleteWebLoginRequest>,
) -> Result<AppResponse<DeleteWebLoginRequest>, ErrorType> {
	trace!("Deleting web login: {}", login_id);

	let rows_affected = query!(
		r#"
		DELETE FROM
			web_login
		WHERE
			login_id = $1 AND
			user_id = $2;
		"#,
		login_id as _,
		user_data.id as _,
	)
	.execute(&mut **database)
	.await?
	.rows_affected();

	if rows_affected == 0 {
		return Err(ErrorType::ResourceDoesNotExist);
	}

	query!(
		r#"
		DELETE FROM
			user_login
		WHERE
			login_id = $1;
		"#,
		login_id as _,
	)
	.execute(&mut **database)
	.await?;

	permissions::mark_login_stale(redis, &login_id).await?;

	AppResponse::builder()
		.body(DeleteWebLoginResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
