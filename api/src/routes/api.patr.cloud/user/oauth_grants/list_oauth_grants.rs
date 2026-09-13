use axum::http::StatusCode;
use models::{api::user::*, utils::TotalCountHeader};

use crate::prelude::*;

pub async fn list_oauth_grants(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListOAuthGrantsPath,
				query:
					ListResourceQueryProcessed {
						sort: _,
						search: _,
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
		user_data,
		state: _,
	}: AuthenticatedAppRequest<'_, ListOAuthGrantsRequest>,
) -> Result<AppResponse<ListOAuthGrantsRequest>, ErrorType> {
	trace!("Listing OAuth grants for user: {}", user_data.id);

	let mut total_count = 0;
	// The join is the point of `oauth_client` existing: a grant renders with
	// the app's name and logo even after that app has left the config.
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
			oauth_client.name AS "client_name",
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
			oauth_login.revoked IS NULL
		ORDER BY
			oauth_login.last_used DESC
		LIMIT $2
		OFFSET $3;
		"#,
		user_data.id as _,
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
				client_name: row.client_name,
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

	AppResponse::builder()
		.body(ListOAuthGrantsResponse { grants })
		.headers(ListOAuthGrantsResponseHeaders {
			total_count: TotalCountHeader(total_count as _),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
