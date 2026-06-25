use axum::http::StatusCode;
use models::api::user::*;

use crate::prelude::*;

/// The handler to search for a user. This will return the user details if
/// found.
pub async fn search_for_user(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: SearchForUserPath,
				query: SearchForUserQueryProcessed { query: user_id },
				headers:
					SearchForUserRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: SearchForUserRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, SearchForUserRequest>,
) -> Result<AppResponse<SearchForUserRequest>, ErrorType> {
	if user_id.trim().len() < 3 {
		return Err(ErrorType::WrongParameters);
	}

	let users = query!(
		r#"
		SELECT
			"user".id,
			"user".username,
			"user".first_name,
			"user".last_name,
			(
				CASE
					WHEN "user".username = $1 THEN 1
					WHEN user_email.email = $1 THEN 1
					WHEN CONCAT(
						'+',
						phone_number_country_code.phone_code,
						user_phone_number.number
					) = $1 THEN 1
					ELSE 0
				END
			) AS score
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
            "user".username ILIKE '%' || $1 || '%' OR
			user_email.email = $1 OR
			CONCAT(
				'+',
				phone_number_country_code.phone_code,
				user_phone_number.number
			) = $1
        ORDER BY
            score DESC
        LIMIT 5;
		"#,
		&user_id,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		WithId::new(
			row.id,
			BasicUserInfo {
				username: row.username,
				first_name: row.first_name,
				last_name: row.last_name,
			},
		)
	})
	.collect();

	AppResponse::builder()
		.body(SearchForUserResponse { users })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
