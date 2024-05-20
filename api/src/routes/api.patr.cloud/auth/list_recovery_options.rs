use axum::http::StatusCode;
use models::api::auth::*;

use crate::prelude::*;

pub async fn list_recovery_options(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: ListRecoveryOptionsPath,
				query: (),
				headers: ListRecoveryOptionsRequestHeaders { user_agent: _ },
				body: ListRecoveryOptionsRequestProcessed { user_id },
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
	}: AppRequest<'_, ListRecoveryOptionsRequest>,
) -> Result<AppResponse<ListRecoveryOptionsRequest>, ErrorType> {
	info!("Listing recovery options for user by query: `{user_id}`");

	let row = query!(
		r#"
		SELECT
			"user".recovery_email,
            phone_number_country_code.phone_code AS "recovery_phone_code",
            "user".recovery_phone_number
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

	let recovery_phone_number = row
		.recovery_phone_number
		.map(|number| {
			let mut chars = number.chars();
			let first = if let Some(first) = chars.next() {
				first
			} else {
				return String::from("******");
			};
			let second = if let Some(first) = chars.next() {
				first
			} else {
				return format!("{first}******");
			};
			let last = if let Some(last) = chars.nth_back(0) {
				last
			} else {
				return format!("{first}{second}******");
			};
			let second_last = if let Some(last) = chars.nth_back(1) {
				last
			} else {
				return format!("{first}{second}******{last}");
			};
			format!("{first}{second}******{second_last}{last}")
		})
		.map(|number| format!("+{}{number}", row.recovery_phone_code));

	let recovery_email = row
		.recovery_email
		.as_deref()
		.and_then(|email| email.split_once('@'))
		.map(|(local, domain)| {
			let local = || {
				if local.is_empty() {
					String::from("*")
				} else if local.len() <= 2 {
					local.chars().map(|_| '*').collect()
				} else if local.len() > 2 && local.len() <= 4 {
					if let Some(first) = local.chars().next() {
						format!("{}***", first)
					} else {
						String::from("*")
					}
				} else {
					let mut chars = local.chars();
					let first = chars.next().unwrap_or('*');
					let last = chars.last().unwrap_or('*');
					format!(
						"{first}{}{last}",
						if local.len() < 10 { "****" } else { "********" }
					)
				}
			};
			format!("{}@{}", local(), domain)
		});

	AppResponse::builder()
		.body(ListRecoveryOptionsResponse {
			recovery_phone_number,
			recovery_email,
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
