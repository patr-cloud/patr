use axum::http::StatusCode;
use models::{api::user::*, utils::TotalCountHeader};
use time::OffsetDateTime;

use crate::prelude::*;

/// List the apps acting on the user's behalf.
pub async fn list_oauth_grants(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListOAuthGrantsPath,
				query:
					ListResourceQueryProcessed {
						sort: _sort_order,
						search:
							UserOAuthGrantSearchParams {
								client_name: client_name_filter,
							},
						count,
						page,
						additional_query: (),
					},
				headers:
					ListOAuthGrantsRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ListOAuthGrantsRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		actor_data,
		state: _,
	}: AuthenticatedAppRequest<'_, ListOAuthGrantsRequest>,
) -> Result<AppResponse<ListOAuthGrantsRequest>, ErrorType> {
	trace!("Listing OAuth grants for user: {}", actor_data.id);

	let now = OffsetDateTime::now_utc();

	let mut total_count = 0;
	// A grant with no live refresh token is over once its last access token
	// expires, revoked or not.
	let grants = query!(
		r#"
		SELECT
			oauth_login.login_id,
			oauth_login.client_id,
			oauth_login.scope,
			oauth_login.created,
			oauth_login.last_used,
			oauth_login.created_ip,
			oauth_login.created_user_agent,
			oauth_client.name,
			oauth_client.logo_url,
			oauth_client.client_uri,
			COUNT(*) OVER() AS "total_count!"
		FROM
			oauth_login
		INNER JOIN
			oauth_client
		ON
			oauth_login.client_id = oauth_client.client_id
		WHERE
			oauth_login.user_id = $1 AND
			oauth_login.revoked IS NULL AND
			(
				oauth_login.last_used > $2 OR
				EXISTS (
					SELECT
						1
					FROM
						oauth_refresh_token
					WHERE
						oauth_refresh_token.login_id = oauth_login.login_id AND
						oauth_refresh_token.consumed IS NULL AND
						oauth_refresh_token.expiry > $3
				)
			) AND
			($4::TEXT IS NULL OR oauth_client.name ILIKE '%' || $4 || '%')
		ORDER BY
			oauth_login.last_used DESC
		LIMIT $5
		OFFSET $6;
		"#,
		actor_data.id,
		now - constants::ACCESS_TOKEN_VALIDITY,
		now,
		client_name_filter,
		count as i32,
		(count * page) as i32,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		total_count = row.total_count;
		WithId::new(
			row.login_id,
			UserOAuthGrant {
				client_id: row.client_id,
				client_name: row.name,
				client_logo_url: row.logo_url,
				client_uri: row.client_uri,
				scope: row.scope,
				created: row.created,
				last_used: row.last_used,
				created_ip: row.created_ip.ip(),
				created_user_agent: row.created_user_agent,
			},
		)
	})
	.collect();

	if page != 0 && total_count == 0 {
		return Err(ErrorType::PageOutOfBounds);
	}

	AppResponse::builder()
		.body(ListOAuthGrantsResponse { grants })
		.headers(ListOAuthGrantsResponseHeaders {
			total_count: TotalCountHeader(total_count as _),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
